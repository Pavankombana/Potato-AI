#![windows_subsystem = "windows"]

mod catalog;
mod config;
mod downloader;
mod engine;
mod models;
mod server;

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

    // Launch UI always in a new tab of the user's browser
    let mut cmd = Command::new("cmd.exe");
    cmd.args(["/c", "start", "", &app_url]);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    if cmd.spawn().is_err() {
        let mut fallback = Command::new("explorer.exe");
        fallback.arg(&app_url);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            fallback.creation_flags(0x08000000);
        }
        let _ = fallback.spawn();
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
