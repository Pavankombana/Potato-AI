use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::catalog::get_curated_models;
use crate::config::{save_config, AppConfig};
use crate::downloader::{DownloadStatus, Downloader};
use crate::engine::{EngineManager, EngineStatus};
use crate::models::{
    browse_gguf_file, delete_model, get_base_dir, get_engine_exe, get_models_dir,
    import_local_model, scan_local_models,
};

const INDEX_HTML: &str = include_str!("../web/index.html");
const LOGO_SVG: &[u8] = include_bytes!("../assets/potato_logo.svg");

pub struct ServerState {
    pub engine: Arc<EngineManager>,
    pub downloader: Downloader,
    pub config: Arc<Mutex<AppConfig>>,
    pub cancel_stream: Arc<AtomicBool>,
    pub last_heartbeat: Arc<Mutex<Instant>>,
    pub should_exit: Arc<AtomicBool>,
}

pub fn start_server(
    engine: Arc<EngineManager>,
    downloader: Downloader,
    config: Arc<Mutex<AppConfig>>,
    last_heartbeat: Arc<Mutex<Instant>>,
    should_exit: Arc<AtomicBool>,
) -> std::io::Result<u16> {
    let listener = match TcpListener::bind("127.0.0.1:8765") {
        Ok(l) => l,
        Err(_) => TcpListener::bind("127.0.0.1:0")?,
    };

    let bound_port = listener.local_addr()?.port();
    let state = Arc::new(ServerState {
        engine,
        downloader,
        config,
        cancel_stream: Arc::new(AtomicBool::new(false)),
        last_heartbeat,
        should_exit,
    });

    std::thread::spawn(move || {
        for stream_res in listener.incoming() {
            if let Ok(stream) = stream_res {
                let st = Arc::clone(&state);
                std::thread::spawn(move || {
                    handle_connection(stream, st);
                });
            }
        }
    });

    Ok(bound_port)
}

fn handle_connection(mut stream: TcpStream, state: Arc<ServerState>) {
    let Ok(read_stream) = stream.try_clone() else {
        return;
    };
    let mut reader = BufReader::new(read_stream);

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() || request_line.is_empty() {
        return;
    }

    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0];
    let full_path = parts[1];
    let path = full_path.split('?').next().unwrap_or(full_path);

    if method == "OPTIONS" {
        let header = "HTTP/1.1 204 No Content\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.flush();
        return;
    }

    let mut content_length: usize = 0;
    loop {
        let mut header_line = String::new();
        if reader.read_line(&mut header_line).is_err() {
            break;
        }
        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some((k, v)) = trimmed.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                if let Ok(len) = v.trim().parse::<usize>() {
                    content_length = len;
                }
            }
        }
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        let _ = reader.read_exact(&mut body);
    }

    match (method, path) {
        ("GET", "/" | "/index.html") => {
            send_response(&mut stream, 200, "text/html; charset=utf-8", INDEX_HTML.as_bytes());
        }
        ("GET", "/api/logo.svg" | "/assets/potato_logo.svg" | "/potato_logo.svg" | "/favicon.ico") => {
            send_response(&mut stream, 200, "image/svg+xml", LOGO_SVG);
        }
        ("POST", "/api/heartbeat") => {
            *state.last_heartbeat.lock().unwrap() = Instant::now();
            send_json(&mut stream, 200, &serde_json::json!({ "ok": true }));
        }
        ("POST", "/api/shutdown") => {
            state.should_exit.store(true, Ordering::SeqCst);
            send_json(&mut stream, 200, &serde_json::json!({ "ok": true }));
        }
        ("GET", "/api/status") => {
            let status_str = match *state.engine.status.lock().unwrap() {
                EngineStatus::Ready => "Ready",
                EngineStatus::Starting | EngineStatus::AutoRecovering => "Starting",
                EngineStatus::Offline => "Offline",
            };
            send_json(&mut stream, 200, &serde_json::json!({ "status": status_str }));
        }
        ("GET", "/api/models") => {
            let installed: Vec<serde_json::Value> = scan_local_models()
                .into_iter()
                .map(|m| {
                    serde_json::json!({
                        "filename": m.filename,
                        "size_mb": m.size_mb,
                    })
                })
                .collect();
            let curated = get_curated_models();
            let active = state.config.lock().unwrap().active_model.clone();

            send_json(
                &mut stream,
                200,
                &serde_json::json!({
                    "installed": installed,
                    "curated": curated,
                    "active": active,
                }),
            );
        }
        ("POST", "/api/models/switch") => {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body) {
                if let Some(filename) = val.get("filename").and_then(|v| v.as_str()) {
                    let model_path = get_models_dir().join(filename);
                    if model_path.exists() {
                        {
                            let mut cfg = state.config.lock().unwrap();
                            cfg.active_model = filename.to_string();
                            save_config(&cfg);
                        }
                        state.engine.start(model_path, get_engine_exe());
                        send_json(&mut stream, 200, &serde_json::json!({ "success": true }));
                        return;
                    }
                }
            }
            send_json(
                &mut stream,
                400,
                &serde_json::json!({ "error": "Model file not found" }),
            );
        }
        ("POST", "/api/models/browse") => {
            if let Some(source_path) = browse_gguf_file() {
                if let Ok(filename) = import_local_model(&source_path) {
                    let model_path = get_models_dir().join(&filename);
                    {
                        let mut cfg = state.config.lock().unwrap();
                        cfg.active_model = filename.clone();
                        save_config(&cfg);
                    }
                    state.engine.start(model_path, get_engine_exe());
                    send_json(
                        &mut stream,
                        200,
                        &serde_json::json!({ "success": true, "filename": filename }),
                    );
                    return;
                }
            }
            send_json(
                &mut stream,
                200,
                &serde_json::json!({ "success": false, "cancelled": true }),
            );
        }
        ("POST", "/api/models/import_path") => {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body) {
                if let Some(path_str) = val.get("path").and_then(|v| v.as_str()) {
                    let p = std::path::PathBuf::from(path_str.trim().trim_matches('"'));
                    if p.exists() && p.is_file() {
                        if let Ok(filename) = import_local_model(&p) {
                            let model_path = get_models_dir().join(&filename);
                            {
                                let mut cfg = state.config.lock().unwrap();
                                cfg.active_model = filename.clone();
                                save_config(&cfg);
                            }
                            state.engine.start(model_path, get_engine_exe());
                            send_json(
                                &mut stream,
                                200,
                                &serde_json::json!({ "success": true, "filename": filename }),
                            );
                            return;
                        }
                    }
                }
            }
            send_json(
                &mut stream,
                400,
                &serde_json::json!({ "error": "Invalid or missing file path" }),
            );
        }
        ("POST", "/api/models/download") => {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body) {
                let url = val.get("url").and_then(|v| v.as_str()).unwrap_or_default();
                let filename = val.get("filename").and_then(|v| v.as_str()).unwrap_or_default();
                if !url.is_empty() && !filename.is_empty() {
                    state.downloader.start_download(
                        url.to_string(),
                        filename.to_string(),
                        get_models_dir(),
                    );
                    send_json(&mut stream, 200, &serde_json::json!({ "success": true }));
                    return;
                }
            }
            send_json(
                &mut stream,
                400,
                &serde_json::json!({ "error": "Invalid download params" }),
            );
        }
        ("POST", "/api/models/download/cancel") => {
            state.downloader.cancel();
            send_json(&mut stream, 200, &serde_json::json!({ "success": true }));
        }
        ("GET", "/api/models/download/status") => {
            let st = state.downloader.status.lock().unwrap().clone();
            let res = match st {
                DownloadStatus::Idle => serde_json::json!({
                    "status": "Idle",
                    "filename": "",
                    "percent": 0.0,
                    "downloaded_bytes": 0,
                    "total_bytes": 0,
                    "speed_mbps": 0.0,
                }),
                DownloadStatus::Downloading {
                    filename,
                    downloaded_bytes,
                    total_bytes,
                    percent,
                    speed_mbps,
                } => serde_json::json!({
                    "status": "Downloading",
                    "filename": filename,
                    "percent": percent,
                    "downloaded_bytes": downloaded_bytes,
                    "total_bytes": total_bytes,
                    "speed_mbps": speed_mbps,
                }),
                DownloadStatus::Completed { filename } => serde_json::json!({
                    "status": "Completed",
                    "filename": filename,
                    "percent": 100.0,
                    "downloaded_bytes": 0,
                    "total_bytes": 0,
                    "speed_mbps": 0.0,
                }),
                DownloadStatus::Failed { filename, error } => serde_json::json!({
                    "status": "Failed",
                    "filename": filename,
                    "error": error,
                    "percent": 0.0,
                    "downloaded_bytes": 0,
                    "total_bytes": 0,
                    "speed_mbps": 0.0,
                }),
            };
            send_json(&mut stream, 200, &res);
        }
        ("POST", "/api/models/delete") => {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body) {
                if let Some(filename) = val.get("filename").and_then(|v| v.as_str()) {
                    let _ = delete_model(filename);
                    send_json(&mut stream, 200, &serde_json::json!({ "success": true }));
                    return;
                }
            }
            send_json(&mut stream, 400, &serde_json::json!({ "error": "Invalid filename" }));
        }
        ("POST", "/api/models/restart") => {
            if let Some(path) = state.engine.active_model_path.lock().unwrap().clone() {
                state.engine.start(path, get_engine_exe());
            }
            send_json(&mut stream, 200, &serde_json::json!({ "success": true }));
        }
        ("GET", "/api/config") => {
            let cfg = state.config.lock().unwrap().clone();
            send_json(
                &mut stream,
                200,
                &serde_json::json!({
                    "system_prompt": cfg.system_prompt,
                    "temperature": cfg.temperature,
                    "cpu_threads": cfg.cpu_threads,
                }),
            );
        }
        ("POST", "/api/config") => {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body) {
                let mut cfg = state.config.lock().unwrap();
                if let Some(p) = val.get("system_prompt").and_then(|v| v.as_str()) {
                    cfg.system_prompt = p.to_string();
                }
                if let Some(t) = val.get("temperature").and_then(|v| v.as_f64()) {
                    cfg.temperature = t as f32;
                }
                if let Some(th) = val.get("cpu_threads").and_then(|v| v.as_u64()) {
                    cfg.cpu_threads = th as usize;
                    *state.engine.threads.lock().unwrap() = th as usize;
                }
                save_config(&cfg);
                send_json(&mut stream, 200, &serde_json::json!({ "success": true }));
                return;
            }
            send_json(&mut stream, 400, &serde_json::json!({ "error": "Invalid config" }));
        }
        ("GET", "/api/hardware") => {
            let parallelism = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
            let threads = *state.engine.threads.lock().unwrap();
            send_json(
                &mut stream,
                200,
                &serde_json::json!({
                    "parallelism": parallelism,
                    "threads": threads,
                    "base_dir": get_base_dir().to_string_lossy(),
                    "engine_exe": get_engine_exe().to_string_lossy(),
                }),
            );
        }
        ("GET", "/api/health") => {
            let healthy = EngineManager::check_health(state.engine.port);
            send_json(&mut stream, 200, &serde_json::json!({ "healthy": healthy }));
        }
        ("POST", "/api/export") => {
            if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&body) {
                if let Some(messages) = val.get("messages").and_then(|v| v.as_array()) {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let filename = format!("chat_export_{}.md", ts);
                    let path = get_base_dir().join(&filename);

                    let mut content = format!("# Potato AI - Chat Export\n\nDate: {}\n\n---\n\n", ts);
                    for m in messages {
                        let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("user");
                        let text = m.get("content").and_then(|c| c.as_str()).unwrap_or("");
                        if role == "user" {
                            content.push_str(&format!("### User\n\n{}\n\n", text));
                        } else {
                            content.push_str(&format!("### Potato AI\n\n{}\n\n---\n\n", text));
                        }
                    }
                    let _ = std::fs::write(&path, content);
                    send_json(
                        &mut stream,
                        200,
                        &serde_json::json!({ "success": true, "filename": filename }),
                    );
                    return;
                }
            }
            send_json(&mut stream, 400, &serde_json::json!({ "error": "Invalid messages" }));
        }
        ("POST", "/api/chat") => {
            handle_chat_stream(&mut stream, &body, &state);
        }
        ("POST", "/api/chat/stop") => {
            state.cancel_stream.store(true, Ordering::SeqCst);
            send_json(&mut stream, 200, &serde_json::json!({ "success": true }));
        }
        ("GET", _) => {
            send_response(&mut stream, 200, "text/html; charset=utf-8", INDEX_HTML.as_bytes());
        }
        _ => {
            send_response(&mut stream, 404, "text/plain; charset=utf-8", b"Not Found");
        }
    }
}

fn handle_chat_stream(write_stream: &mut TcpStream, body: &[u8], state: &ServerState) {
    state.cancel_stream.store(false, Ordering::SeqCst);

    let incoming: Result<serde_json::Value, _> = serde_json::from_slice(body);
    let messages_val = match incoming {
        Ok(ref val) => val
            .get("messages")
            .cloned()
            .unwrap_or(serde_json::Value::Array(Vec::new())),
        Err(_) => serde_json::Value::Array(Vec::new()),
    };

    let mut messages: Vec<serde_json::Value> = match messages_val {
        serde_json::Value::Array(arr) => arr,
        _ => Vec::new(),
    };

    let (system_prompt, temperature) = {
        let cfg = state.config.lock().unwrap();
        (cfg.system_prompt.clone(), cfg.temperature)
    };

    let has_system = messages
        .first()
        .and_then(|m| m.get("role"))
        .and_then(|r| r.as_str())
        == Some("system");

    if !has_system && !system_prompt.is_empty() {
        messages.insert(
            0,
            serde_json::json!({
                "role": "system",
                "content": system_prompt
            }),
        );
    }

    let req_payload = serde_json::json!({
        "messages": messages,
        "temperature": temperature,
        "stream": true
    });

    let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\nAccess-Control-Allow-Origin: *\r\n\r\n";
    if write_stream.write_all(header.as_bytes()).is_err() {
        return;
    }
    let _ = write_stream.flush();

    let engine_url = format!("http://127.0.0.1:{}/v1/chat/completions", state.engine.port);
    let resp = ureq::post(&engine_url)
        .header("Content-Type", "application/json")
        .send_json(&req_payload);

    match resp {
        Ok(r) => {
            let mut reader = BufReader::new(r.into_body().into_reader());
            let mut line = String::new();
            while let Ok(n) = reader.read_line(&mut line) {
                if n == 0 {
                    break;
                }
                if state.cancel_stream.load(Ordering::SeqCst) {
                    break;
                }
                if write_stream.write_all(line.as_bytes()).is_err() {
                    break;
                }
                let _ = write_stream.flush();
                line.clear();
            }
        }
        Err(e) => {
            let err_text = format!(
                "[Model loading or starting: {}. Please wait a few seconds and try again.]",
                e
            );
            let chunk = serde_json::json!({
                "choices": [{
                    "delta": {
                        "content": err_text
                    }
                }]
            });
            let msg = format!(
                "data: {}\n\ndata: [DONE]\n\n",
                serde_json::to_string(&chunk).unwrap_or_default()
            );
            let _ = write_stream.write_all(msg.as_bytes());
            let _ = write_stream.flush();
        }
    }
}

fn send_response(stream: &mut TcpStream, status: u16, content_type: &str, body: &[u8]) {
    let status_text = match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        404 => "Not Found",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let header = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\n\r\n",
        status,
        status_text,
        content_type,
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn send_json(stream: &mut TcpStream, status: u16, val: &serde_json::Value) {
    let body = serde_json::to_vec(val).unwrap_or_default();
    send_response(stream, status, "application/json; charset=utf-8", &body);
}
