use std::path::PathBuf;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LocalModel {
    pub filename: String,
    pub path: PathBuf,
    pub size_mb: u64,
}

pub fn get_base_dir() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            // 1. If parent has models or engine folder, that is the base dir
            if parent.join("models").exists() || parent.join("engine").exists() {
                return parent.to_path_buf();
            }
            // 2. If running from target/debug or target/release/app.exe, walk up
            let mut curr = parent;
            for _ in 0..4 {
                if let Some(p) = curr.parent() {
                    if p.join("models").exists() || p.join("engine").exists() {
                        return p.to_path_buf();
                    }
                    curr = p;
                }
            }
            // 3. If parent is not a cargo build folder, use parent directly
            let dir_name = parent.file_name().unwrap_or_default().to_string_lossy();
            if dir_name != "debug" && dir_name != "release" && dir_name != "target" {
                return parent.to_path_buf();
            }
        }
    }

    // Fallback to current working directory
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn get_models_dir() -> PathBuf {
    let dir = get_base_dir().join("models");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn get_engine_exe() -> PathBuf {
    get_base_dir().join("engine").join("llama-server.exe")
}

pub fn scan_local_models() -> Vec<LocalModel> {
    let models_dir = get_models_dir();
    let mut result = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "gguf" {
                        let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        let size_mb = entry.metadata().map(|m| m.len() / (1024 * 1024)).unwrap_or(0);
                        result.push(LocalModel {
                            filename,
                            path,
                            size_mb,
                        });
                    }
                }
            }
        }
    }

    result.sort_by(|a, b| a.filename.cmp(&b.filename));
    result
}

pub fn delete_model(filename: &str) -> std::io::Result<()> {
    let path = get_models_dir().join(filename);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct OpenFileNameW {
    l_struct_size: u32,
    hwnd_owner: *mut std::ffi::c_void,
    h_instance: *mut std::ffi::c_void,
    lpstr_filter: *const u16,
    lpstr_custom_filter: *mut u16,
    n_max_cust_filter: u32,
    n_filter_index: u32,
    lpstr_file: *mut u16,
    n_max_file: u32,
    lpstr_file_title: *mut u16,
    n_max_file_title: u32,
    lpstr_initial_dir: *const u16,
    lpstr_title: *const u16,
    flags: u32,
    n_file_offset: u16,
    n_file_extension: u16,
    lpstr_def_ext: *const u16,
    l_cust_data: usize,
    lpfn_hook: *mut std::ffi::c_void,
    lp_template_name: *const u16,
    pv_reserved: *mut std::ffi::c_void,
    dw_reserved: u32,
    flags_ex: u32,
}

#[cfg(target_os = "windows")]
#[link(name = "comdlg32")]
#[link(name = "ole32")]
#[link(name = "user32")]
unsafe extern "system" {
    fn GetOpenFileNameW(lpofn: *mut OpenFileNameW) -> i32;
    fn CommDlgExtendedError() -> u32;
    fn CoInitializeEx(pv_reserved: *mut std::ffi::c_void, dw_co_init: u32) -> i32;
    fn CoUninitialize();
    fn GetForegroundWindow() -> *mut std::ffi::c_void;
}

#[cfg(target_os = "windows")]
fn browse_via_powershell() -> Option<PathBuf> {
    use std::os::windows::process::CommandExt;
    let script = r#"[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.OpenFileDialog; $d.Filter = 'GGUF Models (*.gguf)|*.gguf|All Files (*.*)|*.*'; $d.Title = 'Select GGUF Model File'; $d.RestoreDirectory = $true; $f = New-Object System.Windows.Forms.Form; $f.TopMost = $true; if ($d.ShowDialog($f) -eq [System.Windows.Forms.DialogResult]::OK) { Write-Output $d.FileName }"#;
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x08000000)
        .output()
        .ok()?;
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !text.is_empty() {
            let p = PathBuf::from(text);
            if p.exists() && p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

pub fn browse_gguf_file() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        let handle = std::thread::spawn(|| {
            unsafe {
                let _ = CoInitializeEx(std::ptr::null_mut(), 0x2 /* COINIT_APARTMENTTHREADED */);
            }

            let foreground_hwnd = unsafe { GetForegroundWindow() };

            let mut file_buf: [u16; 4096] = [0; 4096];
            let filter: Vec<u16> = "GGUF Model Files (*.gguf)\0*.gguf\0All Files (*.*)\0*.*\0\0"
                .encode_utf16()
                .collect();
            let title: Vec<u16> = "Select GGUF Model File\0".encode_utf16().collect();
            let def_ext: Vec<u16> = "gguf\0".encode_utf16().collect();

            let mut ofn: OpenFileNameW = unsafe { std::mem::zeroed() };
            ofn.l_struct_size = std::mem::size_of::<OpenFileNameW>() as u32;
            ofn.hwnd_owner = foreground_hwnd;
            ofn.lpstr_filter = filter.as_ptr();
            ofn.n_filter_index = 1;
            ofn.lpstr_file = file_buf.as_mut_ptr();
            ofn.n_max_file = file_buf.len() as u32;
            ofn.lpstr_title = title.as_ptr();
            ofn.lpstr_def_ext = def_ext.as_ptr();
            // OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_DONTADDTORECENT
            ofn.flags = 0x00080000 | 0x00001000 | 0x00000800 | 0x00000008 | 0x02000000;

            let success = unsafe { GetOpenFileNameW(&mut ofn) };
            if success != 0 {
                let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
                let path_str = OsString::from_wide(&file_buf[..len]);
                let p = PathBuf::from(path_str);
                unsafe { CoUninitialize(); }
                if p.exists() && p.is_file() {
                    return (Some(p), false);
                }
            }

            let err = unsafe { CommDlgExtendedError() };
            unsafe { CoUninitialize(); }
            // If err == 0, user cancelled. If err != 0, an error occurred in comdlg32.
            (None, err != 0)
        });

        if let Ok((path_opt, has_error)) = handle.join() {
            if path_opt.is_some() {
                return path_opt;
            }
            if !has_error {
                return None;
            }
        }

        // Fallback to PowerShell Windows Forms dialog
        return browse_via_powershell();
    }
    #[allow(unreachable_code)]
    None
}

pub fn import_local_model(source_path: &std::path::Path) -> std::io::Result<String> {
    let filename = source_path
        .file_name()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid filename"))?
        .to_string_lossy()
        .to_string();
    let dest_dir = get_models_dir();
    let _ = std::fs::create_dir_all(&dest_dir);
    let dest = dest_dir.join(&filename);

    let is_same = if let (Ok(c1), Ok(c2)) = (source_path.canonicalize(), dest.canonicalize()) {
        c1 == c2
    } else {
        source_path == dest
    };

    if !is_same && !dest.exists() {
        if std::fs::hard_link(source_path, &dest).is_err() {
            if let Err(e) = std::fs::copy(source_path, &dest) {
                let _ = std::fs::remove_file(&dest);
                return Err(e);
            }
        }
    }
    Ok(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ofn_size() {
        assert_eq!(std::mem::size_of::<OpenFileNameW>(), 152);
    }

    #[test]
    fn test_paths_and_models() {
        let base = get_base_dir();
        assert!(base.exists(), "Base dir must exist");

        let models = get_models_dir();
        assert!(models.exists(), "Models dir must exist");

        let engine = get_engine_exe();
        assert!(engine.to_string_lossy().contains("llama-server.exe"));

        let scanned = scan_local_models();
        for m in scanned {
            assert!(m.filename.ends_with(".gguf"));
        }
    }

    #[test]
    fn test_import_local_model() {
        let temp_dir = std::env::temp_dir().join(format!("potato_ai_test_import_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let sample_file = temp_dir.join("test_model_runner.gguf");
        std::fs::write(&sample_file, b"test gguf content").unwrap();

        let res = import_local_model(&sample_file);
        assert!(res.is_ok());
        let imported_name = res.unwrap();
        assert_eq!(imported_name, "test_model_runner.gguf");

        let dest = get_models_dir().join("test_model_runner.gguf");
        assert!(dest.exists());
        let _ = std::fs::remove_file(dest);
        let _ = std::fs::remove_dir_all(temp_dir);
    }
}


