#![windows_subsystem = "windows"]

mod catalog;
mod config;
mod downloader;
mod engine;
mod models;
mod server;
mod sysinfo;

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use downloader::Downloader;
use engine::EngineManager;

fn main() {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        EngineManager::kill_all_stale();
        default_panic(panic_info);
    }));

    EngineManager::kill_all_stale();

    let mut config = config::load_config();
    let specs = sysinfo::get_system_specs();
    if config.cpu_threads == 0 || config.cpu_threads > specs.logical_cores {
        config.cpu_threads = specs.recommended_threads;
        config::save_config(&config);
    }

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
    let base_cache = models::get_base_dir().join("cache").join("profile");
    let profile_dir = if std::fs::create_dir_all(&base_cache).is_ok() {
        base_cache
    } else {
        let fallback = std::env::temp_dir().join("PotatoAI").join("profile");
        let _ = std::fs::create_dir_all(&fallback);
        fallback
    };

    // Launch UI as a standalone desktop app window (Edge, Chrome, Brave, or system default browser)
    let mut browser_candidates = vec![
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe".to_string(),
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe".to_string(),
        r"C:\Program Files\Google\Chrome\Application\chrome.exe".to_string(),
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe".to_string(),
        r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe".to_string(),
    ];

    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        browser_candidates.push(format!(r"{}\Microsoft\Edge\Application\msedge.exe", local));
        browser_candidates.push(format!(r"{}\Google\Chrome\Application\chrome.exe", local));
        browser_candidates.push(format!(r"{}\BraveSoftware\Brave-Browser\Application\brave.exe", local));
    }

    let mut launched = false;
    for path in &browser_candidates {
        let p = std::path::PathBuf::from(path);
        if p.exists() {
            let mut cmd = Command::new(p);
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
            if cmd.spawn().is_ok() {
                launched = true;
                break;
            }
        }
    }

    if !launched {
        // Universal fallback to default Windows browser
        let mut cmd = Command::new("cmd.exe");
        cmd.args(["/c", "start", "", &app_url]);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let _ = cmd.spawn();
    }

    // Grace period for browser window to launch and send first heartbeat
    let startup_time = Instant::now();
    let mut initial_heartbeat_received = false;

    // Up to 60 seconds grace period on low-spec systems for browser to launch and connect
    for _ in 0..120 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if should_exit.load(Ordering::SeqCst) {
            break;
        }
        let last = *last_heartbeat.lock().unwrap();
        if last > startup_time {
            initial_heartbeat_received = true;
            break;
        }
    }

    if initial_heartbeat_received {
        // Monitor heartbeat: Keep running as long as the user has the window open
        // 60-second inactivity window accommodates background tab throttling / sleeping tabs
        loop {
            std::thread::sleep(std::time::Duration::from_millis(1000));
            if should_exit.load(Ordering::SeqCst) {
                break;
            }
            let elapsed = last_heartbeat.lock().unwrap().elapsed();
            if elapsed > std::time::Duration::from_secs(60) {
                // Window was closed or destroyed
                break;
            }
        }
    }

    engine.stop();
    EngineManager::kill_all_stale();
}
