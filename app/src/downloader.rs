use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone)]
pub enum DownloadStatus {
    Idle,
    Downloading {
        filename: String,
        downloaded_bytes: u64,
        total_bytes: u64,
        percent: f32,
        speed_mbps: f32,
    },
    Completed {
        filename: String,
    },
    Failed {
        filename: String,
        error: String,
    },
}

#[derive(Clone)]
pub struct Downloader {
    pub status: Arc<Mutex<DownloadStatus>>,
    pub cancel_flag: Arc<AtomicBool>,
}

impl Downloader {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(DownloadStatus::Idle)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start_download(&self, url: String, filename: String, destination_dir: PathBuf) {
        let status_arc = Arc::clone(&self.status);
        let cancel_arc = Arc::clone(&self.cancel_flag);
        cancel_arc.store(false, Ordering::SeqCst);

        std::thread::spawn(move || {
            let _ = std::fs::create_dir_all(&destination_dir);
            let dest_file = destination_dir.join(&filename);
            let temp_file = destination_dir.join(format!("{}.download", &filename));

            *status_arc.lock().unwrap() = DownloadStatus::Downloading {
                filename: filename.clone(),
                downloaded_bytes: 0,
                total_bytes: 0,
                percent: 0.0,
                speed_mbps: 0.0,
            };

            let mut current_url = url;
            let mut final_resp = None;

            for _ in 0..10 {
                let r = match ureq::get(&current_url)
                    .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) PotatoAI")
                    .call()
                {
                    Ok(res) => res,
                    Err(e) => {
                        *status_arc.lock().unwrap() = DownloadStatus::Failed {
                            filename,
                            error: format!("Network error: {}", e),
                        };
                        return;
                    }
                };

                let code = r.status().as_u16();
                if (300..400).contains(&code) {
                    if let Some(loc) = r.headers().get("location").and_then(|v| v.to_str().ok()) {
                        if loc.starts_with('/') {
                            if let Some(pos) = current_url.find("://") {
                                let after_scheme = &current_url[pos + 3..];
                                let host = after_scheme.split('/').next().unwrap_or("");
                                current_url = format!("{}{}{}", &current_url[..pos + 3], host, loc);
                            } else {
                                current_url = loc.to_string();
                            }
                        } else {
                            current_url = loc.to_string();
                        }
                        continue;
                    }
                }

                final_resp = Some(r);
                break;
            }

            let resp = match final_resp {
                Some(r) => r,
                None => {
                    *status_arc.lock().unwrap() = DownloadStatus::Failed {
                        filename,
                        error: "Too many redirects".to_string(),
                    };
                    return;
                }
            };

            let total_bytes: u64 = resp
                .headers()
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            *status_arc.lock().unwrap() = DownloadStatus::Downloading {
                filename: filename.clone(),
                downloaded_bytes: 0,
                total_bytes,
                percent: 0.0,
                speed_mbps: 0.0,
            };

            let mut reader = resp.into_body().into_reader();
            let mut file = match std::fs::File::create(&temp_file) {
                Ok(f) => f,
                Err(e) => {
                    *status_arc.lock().unwrap() = DownloadStatus::Failed {
                        filename,
                        error: format!("Cannot create file on disk: {}", e),
                    };
                    return;
                }
            };

            let mut buffer = [0u8; 256 * 1024]; // 256KB buffer
            let mut downloaded_bytes: u64 = 0;
            let start_time = Instant::now();
            let mut last_update = Instant::now();

            loop {
                if cancel_arc.load(Ordering::SeqCst) {
                    drop(file);
                    let _ = std::fs::remove_file(&temp_file);
                    *status_arc.lock().unwrap() = DownloadStatus::Idle;
                    return;
                }

                match reader.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if let Err(e) = file.write_all(&buffer[..n]) {
                            drop(file);
                            let _ = std::fs::remove_file(&temp_file);
                            *status_arc.lock().unwrap() = DownloadStatus::Failed {
                                filename,
                                error: format!("Disk write error: {}", e),
                            };
                            return;
                        }
                        downloaded_bytes += n as u64;

                        // Update status every 100ms
                        if last_update.elapsed().as_millis() >= 100 {
                            let elapsed_secs = start_time.elapsed().as_secs_f32().max(0.001);
                            let speed_mbps = (downloaded_bytes as f32 / (1024.0 * 1024.0)) / elapsed_secs;
                            let percent = if total_bytes > 0 {
                                (downloaded_bytes as f32 / total_bytes as f32) * 100.0
                            } else {
                                0.0
                            };

                            *status_arc.lock().unwrap() = DownloadStatus::Downloading {
                                filename: filename.clone(),
                                downloaded_bytes,
                                total_bytes,
                                percent,
                                speed_mbps,
                            };
                            last_update = Instant::now();
                        }
                    }
                    Err(e) => {
                        drop(file);
                        let _ = std::fs::remove_file(&temp_file);
                        *status_arc.lock().unwrap() = DownloadStatus::Failed {
                            filename,
                            error: format!("Download interrupted: {}", e),
                        };
                        return;
                    }
                }
            }

            // Finished, rename temp file
            drop(file);
            let _ = std::fs::remove_file(&dest_file);
            if let Err(e) = std::fs::rename(&temp_file, &dest_file) {
                *status_arc.lock().unwrap() = DownloadStatus::Failed {
                    filename,
                    error: format!("Failed to finalize model file: {}", e),
                };
                return;
            }

            *status_arc.lock().unwrap() = DownloadStatus::Completed { filename };
        });
    }

    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }
}
