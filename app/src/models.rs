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
            // Check if models folder exists in current exe parent
            if parent.join("models").exists() {
                return parent.to_path_buf();
            }
            // Check if exe is inside target/debug or target/release or app/
            if let Some(grandparent) = parent.parent() {
                if grandparent.join("models").exists() {
                    return grandparent.to_path_buf();
                }
                if let Some(great_grandparent) = grandparent.parent() {
                    if great_grandparent.join("models").exists() {
                        return great_grandparent.to_path_buf();
                    }
                    if let Some(g4) = great_grandparent.parent() {
                        if g4.join("models").exists() {
                            return g4.to_path_buf();
                        }
                    }
                }
            }
        }
    }

    // Fallback to current working directory
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn get_models_dir() -> PathBuf {
    get_base_dir().join("models")
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
#[link(name = "user32")]
unsafe extern "system" {
    fn GetForegroundWindow() -> *mut std::ffi::c_void;
    fn GetOpenFileNameW(lpofn: *mut OpenFileNameW) -> i32;
}

pub fn browse_gguf_file() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;

        let mut file_buf: [u16; 512] = [0; 512];
        let filter: Vec<u16> = "GGUF Model Files (*.gguf)\0*.gguf\0All Files (*.*)\0*.*\0\0"
            .encode_utf16()
            .collect();
        let title: Vec<u16> = "Select GGUF Model File\0".encode_utf16().collect();

        let mut ofn: OpenFileNameW = unsafe { std::mem::zeroed() };
        ofn.l_struct_size = std::mem::size_of::<OpenFileNameW>() as u32;
        ofn.hwnd_owner = unsafe { GetForegroundWindow() };
        ofn.lpstr_filter = filter.as_ptr();
        ofn.lpstr_file = file_buf.as_mut_ptr();
        ofn.n_max_file = file_buf.len() as u32;
        ofn.lpstr_title = title.as_ptr();
        // OFN_EXPLORER | OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST
        ofn.flags = 0x00080000 | 0x00001000 | 0x00000800;

        let success = unsafe { GetOpenFileNameW(&mut ofn) };
        if success != 0 {
            let len = file_buf.iter().position(|&c| c == 0).unwrap_or(file_buf.len());
            let path_str = OsString::from_wide(&file_buf[..len]);
            let p = PathBuf::from(path_str);
            if p.exists() && p.is_file() {
                return Some(p);
            }
        }
    }
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
    if !dest.exists() && source_path != dest {
        std::fs::copy(source_path, &dest)?;
    }
    Ok(filename)
}

