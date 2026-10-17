use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    #[cfg(target_os = "windows")]
    {
        let out_dir = env::var("OUT_DIR").unwrap();
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
        let icon_path = PathBuf::from(&manifest_dir).parent().unwrap().join("potato_ai.ico");

        if icon_path.exists() {
            let rc_content = format!("1 ICON \"{}\"\n", icon_path.display().to_string().replace('\\', "/"));
            let rc_path = PathBuf::from(&out_dir).join("icon.rc");
            let res_path = PathBuf::from(&out_dir).join("icon.o");

            if std::fs::write(&rc_path, rc_content).is_ok() {
                let mut windres_candidates = vec![
                    "windres".to_string(),
                    r"P:\mingw64\bin\windres.exe".to_string(),
                    r"C:\mingw64\bin\windres.exe".to_string(),
                ];
                if let Ok(local) = env::var("LOCALAPPDATA") {
                    let winget_dir = PathBuf::from(local).join(r"Microsoft\WinGet\Packages");
                    if let Ok(entries) = std::fs::read_dir(&winget_dir) {
                        for entry in entries.flatten() {
                            let path = entry.path();
                            if path.to_string_lossy().contains("MinGW") {
                                for sub in &[
                                    path.join("bin").join("windres.exe"),
                                    path.join("mingw64").join("bin").join("windres.exe"),
                                ] {
                                    if sub.exists() {
                                        windres_candidates.push(sub.to_string_lossy().to_string());
                                    }
                                }
                                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                                    for sub_entry in sub_entries.flatten() {
                                        let p = sub_entry.path().join("bin").join("windres.exe");
                                        if p.exists() {
                                            windres_candidates.push(p.to_string_lossy().to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                let mut compiled = false;
                for wr in &windres_candidates {
                    if let Ok(status) = Command::new(wr)
                        .args([
                            "-i",
                            &rc_path.to_string_lossy(),
                            "-O",
                            "coff",
                            "-o",
                            &res_path.to_string_lossy(),
                        ])
                        .status()
                    {
                        if status.success() {
                            compiled = true;
                            break;
                        }
                    }
                }

                if !compiled {
                    if let Ok(status) = Command::new("rc.exe")
                        .args(["/fo", &res_path.to_string_lossy(), &rc_path.to_string_lossy()])
                        .status()
                    {
                        if status.success() {
                            compiled = true;
                        }
                    }
                }

                if compiled {
                    println!("cargo:rustc-link-arg={}", res_path.display());
                }
            }
        }
        println!("cargo:rerun-if-changed=../potato_ai.ico");
        println!("cargo:rerun-if-changed=build.rs");
    }
}
