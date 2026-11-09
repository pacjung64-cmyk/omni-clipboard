use crate::ai_lens::{LensAction, LensClient};
use crate::listener::Win32Listener;
use crate::models::SearchQuery;
use crate::storage::ClipStorage;
use std::sync::Arc;
use tiny_http::{Header, Response, Server, StatusCode};

pub struct AppServer {
    storage: Arc<dyn ClipStorage>,
    ai_client: Arc<LensClient>,
    port: u16,
}

impl AppServer {
    pub fn new(storage: Arc<dyn ClipStorage>, ai_client: Arc<LensClient>, port: u16) -> Self {
        Self {
            storage,
            ai_client,
            port,
        }
    }

    pub fn start(&self) -> Result<(), String> {
        let addr = format!("127.0.0.1:{}", self.port);
        let server = Server::http(&addr).map_err(|e| e.to_string())?;
        println!("⚡ Omni-Clipboard Server running on http://{}", addr);

        let storage = Arc::clone(&self.storage);
        let ai_client = Arc::clone(&self.ai_client);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();

            for mut request in server.incoming_requests() {
                let url = request.url().to_string();
                let method = request.method().as_str().to_uppercase();

                if method == "GET" && (url == "/" || url == "/index.html") {
                    let html = include_str!("ui.html");
                    let header = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap();
                    let response = Response::from_string(html).with_header(header);
                    let _ = request.respond(response);
                    continue;
                }

                if method == "GET" && url.starts_with("/api/search") {
                    let query_str = url.split("?q=").nth(1).unwrap_or("");
                    let decoded_query = urlencoding_decode(query_str);

                    let results = storage.search_clips(&SearchQuery {
                        query: decoded_query,
                        pinned_only: false,
                        limit: 50,
                    }).unwrap_or_default();

                    let json = serde_json::to_string(&results).unwrap_or_else(|_| "[]".into());
                    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    let response = Response::from_string(json).with_header(header);
                    let _ = request.respond(response);
                    continue;
                }

                if method == "POST" && url == "/api/paste" {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);

                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(content) = val["content"].as_str() {
                            if let Ok(mut cb) = arboard::Clipboard::new() {
                                let _ = cb.set_text(content);
                            }
                            // Simulate paste into background app
                            Win32Listener::simulate_paste();
                        }
                    }

                    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    let response = Response::from_string("{\"status\":\"ok\"}").with_header(header);
                    let _ = request.respond(response);
                    continue;
                }

                if method == "POST" && url == "/api/pin" {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);

                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let (Some(id), Some(pinned)) = (val["id"].as_str(), val["pinned"].as_bool()) {
                            let _ = storage.set_pinned(id, pinned);
                        }
                    }

                    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    let response = Response::from_string("{\"status\":\"ok\"}").with_header(header);
                    let _ = request.respond(response);
                    continue;
                }

                if method == "POST" && url == "/api/delete" {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);

                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(id) = val["id"].as_str() {
                            let _ = storage.delete_clip(id);
                        }
                    }

                    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    let response = Response::from_string("{\"status\":\"ok\"}").with_header(header);
                    let _ = request.respond(response);
                    continue;
                }

                if method == "POST" && url == "/api/ai_lens" {
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);

                    let mut result_json = serde_json::json!({"error": "Invalid request"});

                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&body) {
                        let action_str = val["action"].as_str().unwrap_or("Explain");
                        let input_text = val["text"].as_str().unwrap_or("");

                        let action = if action_str.starts_with("Custom:") {
                            LensAction::Custom(action_str.trim_start_matches("Custom:").to_string())
                        } else {
                            match action_str {
                                "TranslateThEn" => LensAction::TranslateThEn,
                                "FixCode" => LensAction::FixCode,
                                "PolishTone" => LensAction::PolishTone,
                                _ => LensAction::Explain,
                            }
                        };

                        let ai = Arc::clone(&ai_client);
                        let text_clone = input_text.to_string();

                        let res = rt.block_on(async move {
                            ai.process_async(&action, &text_clone).await
                        });

                        match res {
                            Ok(text) => result_json = serde_json::json!({"result": text}),
                            Err(e) => result_json = serde_json::json!({"error": e}),
                        }
                    }

                    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap();
                    let response = Response::from_string(result_json.to_string()).with_header(header);
                    let _ = request.respond(response);
                    continue;
                }

                let response = Response::from_string("Not Found").with_status_code(StatusCode(404));
                let _ = request.respond(response);
            }
        });

        Ok(())
    }

    pub fn launch_spotlight_window(port: u16) {
        let edge_path = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe";
        let app_url = format!("--app=http://127.0.0.1:{}", port);
        
        let _ = std::process::Command::new(edge_path)
            .args([
                &app_url,
                "--window-size=720,520",
                "--app-window-style=popup",
                "--disable-features=TranslateUI",
            ])
            .spawn();
    }
}

fn urlencoding_decode(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            let hex_str = format!("{}{}", h1, h2);
            if let Ok(byte) = u8::from_str_radix(&hex_str, 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}
