#![windows_subsystem = "windows"]

mod catalog;
mod config;
mod downloader;
mod engine;
mod models;
mod server;

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use downloader::Downloader;
use engine::EngineManager;

fn find_edge_path() -> Option<PathBuf> {
    let candidates = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ];
    for path in &candidates {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let p = PathBuf::from(local).join(r"Microsoft\Edge\Application\msedge.exe");
        if p.exists() {
            return Some(p);
        }
    }

    let chrome_candidates = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];
    for path in &chrome_candidates {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }

    None
}

fn main() {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        EngineManager::kill_all_stale();
        default_panic(panic_info);
    }));

    EngineManager::kill_all_stale();

    let mut config = config::load_config();
    let local_models = models::scan_local_models();
    if (config.active_model.is_empty()
        || !local_models.iter().any(|m| m.filename == config.active_model))
        && !local_models.is_empty()
    {
        config.active_model = local_models[0].filename.clone();
        config::save_config(&config);
    }

    let engine_port = 8080;
    let engine = Arc::new(EngineManager::new(engine_port));
    *engine.threads.lock().unwrap() = config.cpu_threads;

    let engine_exe = models::get_engine_exe();

    if !config.active_model.is_empty() {
        let model_path = models::get_models_dir().join(&config.active_model);
        if model_path.exists() && engine_exe.exists() {
            engine.start(model_path, engine_exe.clone());
        }
    }

    if engine_exe.exists() {
        engine.start_auto_healer(engine_exe);
    }

    let downloader = Downloader::new();
    let config_arc = Arc::new(Mutex::new(config));
    let last_heartbeat = Arc::new(Mutex::new(Instant::now()));
    let should_exit = Arc::new(AtomicBool::new(false));

    let server_port = match server::start_server(
        Arc::clone(&engine),
        downloader,
        Arc::clone(&config_arc),
        Arc::clone(&last_heartbeat),
        Arc::clone(&should_exit),
    ) {
        Ok(port) => port,
        Err(_) => 8765,
    };

    std::thread::sleep(std::time::Duration::from_millis(150));

    let app_url = format!("http://127.0.0.1:{}", server_port);
    let profile_dir = models::get_base_dir().join("cache").join("profile");
    let _ = std::fs::create_dir_all(&profile_dir);

    // Launch UI in Edge App-Mode or default browser
    if let Some(edge_path) = find_edge_path() {
        let mut cmd = Command::new(edge_path);
        cmd.args([
            &format!("--app={}", app_url),
            "--start-maximized",
            &format!("--user-data-dir={}", profile_dir.to_string_lossy()),
            "--no-first-run",
            "--no-default-browser-check",
        ]);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let _ = cmd.spawn();
    } else {
        let mut cmd = Command::new("explorer.exe");
        cmd.arg(&app_url);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let _ = cmd.status();
    }

    // Grace period for browser window to launch and send first heartbeat
    for _ in 0..12 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if should_exit.load(Ordering::SeqCst) {
            break;
        }
    }

    // Monitor heartbeat: Keep running as long as the user has the window open
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if should_exit.load(Ordering::SeqCst) {
            break;
        }
        let elapsed = last_heartbeat.lock().unwrap().elapsed();
        if elapsed > std::time::Duration::from_secs(8) {
            // Window was closed or destroyed
            break;
        }
    }

    engine.stop();
    EngineManager::kill_all_stale();
}
