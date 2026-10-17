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

pub fn browse_gguf_file() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let script = r#"
Add-Type -AssemblyName System.Windows.Forms
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Title = "Select GGUF Model File"
$dialog.Filter = "GGUF Models (*.gguf)|*.gguf|All Files (*.*)|*.*"
$dummy = New-Object System.Windows.Forms.Form
$dummy.TopMost = $true
if ($dialog.ShowDialog($dummy) -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    [Console]::WriteLine($dialog.FileName)
}
"#;
        let output = std::process::Command::new("powershell")
            .args(["-STA", "-NoProfile", "-Command", script])
            .creation_flags(0x08000000)
            .output()
            .ok()?;
        let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path_str.is_empty() {
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

