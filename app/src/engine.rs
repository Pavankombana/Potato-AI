use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineStatus {
    Offline,
    Starting,
    Ready,
    AutoRecovering,
}

pub struct EngineManager {
    pub status: Arc<Mutex<EngineStatus>>,
    pub active_model_path: Arc<Mutex<Option<PathBuf>>>,
    server_process: Arc<Mutex<Option<Child>>>,
    pub port: u16,
    pub threads: Arc<Mutex<usize>>,
}

impl EngineManager {
    pub fn new(port: u16) -> Self {
        let default_threads = match std::thread::available_parallelism() {
            Ok(n) => {
                let count = n.get();
                if count > 4 { count - 1 } else { count.max(2) }
            }
            Err(_) => 4,
        };
        Self {
            status: Arc::new(Mutex::new(EngineStatus::Offline)),
            active_model_path: Arc::new(Mutex::new(None)),
            server_process: Arc::new(Mutex::new(None)),
            port,
            threads: Arc::new(Mutex::new(default_threads)),
        }
    }

    pub fn kill_all_stale() {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let mut cmd = Command::new("taskkill");
            cmd.args(["/F", "/IM", "llama-server.exe", "/T"])
                .creation_flags(CREATE_NO_WINDOW);
            let _ = cmd.output();
        }
    }

    pub fn check_health(port: u16) -> bool {
        let url = format!("http://127.0.0.1:{}/health", port);
        if let Ok(resp) = ureq::get(&url).call() {
            return resp.status().as_u16() == 200;
        }
        false
    }

    pub fn start(&self, model_path: PathBuf, engine_exe: PathBuf) {
        *self.active_model_path.lock().unwrap() = Some(model_path.clone());
        let status_arc = Arc::clone(&self.status);
        let process_arc = Arc::clone(&self.server_process);
        let threads_arc = Arc::clone(&self.threads);
        let port = self.port;

        std::thread::spawn(move || {
            *status_arc.lock().unwrap() = EngineStatus::Starting;

            // Kill any existing instance
            if let Some(mut child) = process_arc.lock().unwrap().take() {
                let _ = child.kill();
            }
            Self::kill_all_stale();

            let threads = *threads_arc.lock().unwrap();

            let engine_dir = engine_exe.parent().unwrap_or(&engine_exe);

            #[cfg(target_os = "windows")]
            let mut cmd = Command::new(&engine_exe);
            cmd.current_dir(engine_dir)
                .args([
                    "-m",
                    &model_path.to_string_lossy(),
                    "-c",
                    "2048",
                    "-t",
                    &threads.to_string(),
                    "--port",
                    &port.to_string(),
                    "--host",
                    "127.0.0.1",
                ]);

            // Hide console window on Windows
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }

            match cmd.spawn() {
                Ok(child) => {
                    *process_arc.lock().unwrap() = Some(child);

                    // Wait for health endpoint to respond
                    for _ in 0..60 {
                        // 15 seconds max wait
                        std::thread::sleep(Duration::from_millis(250));
                        if Self::check_health(port) {
                            *status_arc.lock().unwrap() = EngineStatus::Ready;
                            return;
                        }
                    }
                    *status_arc.lock().unwrap() = EngineStatus::Offline;
                }
                Err(_) => {
                    *status_arc.lock().unwrap() = EngineStatus::Offline;
                }
            }
        });
    }

    pub fn start_auto_healer(&self, engine_exe: PathBuf) {
        let status_arc = Arc::clone(&self.status);
        let model_arc = Arc::clone(&self.active_model_path);
        let proc_arc = Arc::clone(&self.server_process);
        let threads_arc = Arc::clone(&self.threads);
        let port = self.port;

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(3));

                let current_status = *status_arc.lock().unwrap();
                if current_status == EngineStatus::Starting {
                    continue;
                }

                let is_healthy = Self::check_health(port);
                if is_healthy {
                    if current_status != EngineStatus::Ready {
                        *status_arc.lock().unwrap() = EngineStatus::Ready;
                    }
                } else if current_status == EngineStatus::Ready {
                    // Server crashed or was killed, auto-recover!
                    *status_arc.lock().unwrap() = EngineStatus::AutoRecovering;

                    if let Some(model_path) = model_arc.lock().unwrap().clone() {
                        if model_path.exists() && engine_exe.exists() {
                            Self::kill_all_stale();
                            let threads = *threads_arc.lock().unwrap();

                            let mut cmd = Command::new(&engine_exe);
                            cmd.current_dir(engine_exe.parent().unwrap_or(&engine_exe))
                                .args([
                                    "-m",
                                    &model_path.to_string_lossy(),
                                    "-c",
                                    "2048",
                                    "-t",
                                    &threads.to_string(),
                                    "--port",
                                    &port.to_string(),
                                    "--host",
                                    "127.0.0.1",
                                ]);

                            #[cfg(target_os = "windows")]
                            {
                                use std::os::windows::process::CommandExt;
                                const CREATE_NO_WINDOW: u32 = 0x08000000;
                                cmd.creation_flags(CREATE_NO_WINDOW);
                            }

                            if let Ok(child) = cmd.spawn() {
                                *proc_arc.lock().unwrap() = Some(child);
                            }
                        }
                    }
                }
            }
        });
    }

    pub fn stop(&self) {
        *self.status.lock().unwrap() = EngineStatus::Offline;
        if let Some(mut child) = self.server_process.lock().unwrap().take() {
            let _ = child.kill();
        }
        Self::kill_all_stale();
    }
}

impl Drop for EngineManager {
    fn drop(&mut self) {
        self.stop();
    }
}
