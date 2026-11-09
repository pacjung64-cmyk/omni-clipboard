use omni_clipboard::ai_lens::{AiProvider, LensClient};
use omni_clipboard::listener::{SystemEvent, Win32Listener};
use omni_clipboard::models::{ClipItem, ContentType};
use omni_clipboard::sanitizer::Sanitizer;
use omni_clipboard::server::AppServer;
use omni_clipboard::storage::{ClipStorage, SqliteStorage};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("============================================================");
    println!("⚡ Omni-Clipboard & Instant AI Lens for Windows");
    println!("============================================================");

    // 1. Initialize persistent SQLite storage
    let storage: Arc<dyn ClipStorage> = Arc::new(SqliteStorage::new("omniclip.db")?);
    println!("💾 SQLite database connected (omniclip.db)");

    // 2. Initialize AI Lens Client (Ollama by default, or Gemini / Groq if env provided)
    let ai_provider = if let Ok(key) = std::env::var("GEMINI_API_KEY") {
        println!("✨ AI Lens: using Gemini 1.5 Flash (via GEMINI_API_KEY)");
        AiProvider::Gemini {
            api_key: key,
            model: "gemini-1.5-flash".into(),
        }
    } else if let Ok(key) = std::env::var("GROQ_API_KEY") {
        println!("⚡ AI Lens: using Groq Llama 3.3 70B (via GROQ_API_KEY)");
        AiProvider::Groq {
            api_key: key,
            model: "llama-3.3-70b-versatile".into(),
        }
    } else {
        println!("🦙 AI Lens: using local Ollama (http://localhost:11434, llama3.2)");
        AiProvider::default()
    };

    let ai_client = Arc::new(LensClient::new(ai_provider));

    // 3. Start local Web Server for Spotlight UI
    let port = 42069;
    let server = AppServer::new(Arc::clone(&storage), Arc::clone(&ai_client), port);
    server.start()?;

    // 4. Start Win32 Clipboard Listener & System Hotkeys
    let listener = Win32Listener::new();
    let storage_clone = Arc::clone(&storage);

    println!("\n🔥 Hotkeys Active:");
    println!("   - [Ctrl + Shift + V] : Open Spotlight Clipboard History");
    println!("   - [Alt + Space]      : Open Instant AI Lens");
    println!("   - URL: http://127.0.0.1:{}\n", port);

    listener.start(move |event| match event {
        SystemEvent::ClipboardUpdated { text, source_app } => {
            let content_type = Sanitizer::classify_and_filter(&text);
            let is_sensitive = matches!(content_type, ContentType::Sensitive);
            let clip = ClipItem::new(&text, content_type, source_app.clone());

            match storage_clone.save_clip(clip) {
                Ok(id) => {
                    let app_str = source_app.unwrap_or_else(|| "Unknown".into());
                    if is_sensitive {
                        println!("🔒 Captured [SECRET - Redacted] from {} (id: {})", app_str, &id[..8]);
                    } else {
                        let preview = if text.len() > 60 {
                            format!("{}...", &text[..57].replace('\n', " "))
                        } else {
                            text.replace('\n', " ")
                        };
                        println!("📋 Captured: \"{}\" from {} (id: {})", preview, app_str, &id[..8]);
                    }
                }
                Err(e) => eprintln!("❌ Failed to save clip: {}", e),
            }
        }
        SystemEvent::HistoryHotkeyPressed => {
            println!("🚀 Hotkey triggered: [Ctrl + Shift + V] -> Opening Spotlight UI");
            AppServer::launch_spotlight_window(port);
        }
        SystemEvent::AiLensHotkeyPressed => {
            println!("✨ Hotkey triggered: [Alt + Space] -> Opening AI Lens");
            AppServer::launch_spotlight_window(port);
        }
    })?;

    println!("App running in background. Press Ctrl+C in terminal to exit.\n");

    // Launch UI on start once to verify
    AppServer::launch_spotlight_window(port);

    // Keep main thread alive
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
