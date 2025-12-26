use enigo::{Enigo, KeyboardControllable, Key};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri::{Emitter, Manager};
use std::thread;
use std::time::Duration;

#[tauri::command]
async fn finalize_and_type(
    app_handle: tauri::AppHandle, 
    raw_text: String, 
    gemini_key: String
) -> Result<(), String> {
    if raw_text.trim().is_empty() { return Ok(()); }

    println!("--- RUST: Processing ---");

    // DEBUG: Verify Key
    if gemini_key.len() > 5 {
        println!("DEBUG: Key loaded (Starts with: {}...)", &gemini_key[..5]);
    } else {
        println!("❌ ERROR: API Key is missing! Check .env file.");
        return Ok(());
    }

    let client = reqwest::Client::new();
    
    // ------------------------------------------------------------------
    // UPDATE: Using 'gemini-2.5-flash' (Stable)
    // This matches the JavaScript documentation you provided.
    // ------------------------------------------------------------------
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}", 
        gemini_key
    );

    let prompt = format!(
        "Fix grammar and remove fillers. Output ONLY the polished text:\n\n{}",
        raw_text
    );

    let body = serde_json::json!({
        "contents": [{ "parts": [{ "text": prompt }] }]
    });

    let mut polished_text = raw_text.clone();

    // CALL API
    match client.post(url).json(&body).send().await {
        Ok(res) => {
            let status = res.status();
            if status.is_success() {
                let json: serde_json::Value = res.json().await.unwrap_or(serde_json::json!({}));
                if let Some(text) = json["candidates"][0]["content"]["parts"][0]["text"].as_str() {
                    polished_text = text.trim().to_string();
                    println!("✅ GEMINI 2.5 SUCCESS: {}", polished_text);
                }
            } else {
                let error_body = res.text().await.unwrap_or_default();
                println!("❌ API ERROR. Status: {}. Message: {}", status, error_body);
                // Fallback to raw text is handled by initialization
            }
        },
        Err(e) => println!("❌ NETWORK ERROR: {}", e),
    }

    // FOCUS FIX: Minimize & Hide
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.minimize();
        let _ = window.hide();
    }
    
    thread::sleep(Duration::from_millis(800));

    // VIRTUAL TYPING
    let mut enigo = Enigo::new();
    enigo.key_up(Key::Control);
    enigo.key_up(Key::Shift);
    
    for char in polished_text.chars() {
        enigo.key_sequence(&char.to_string());
        thread::sleep(Duration::from_millis(5));
    }

    println!("--- RUST: Typing Complete ---");
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        // ----------------------------------------------------------
        // CRITICAL FIX: The Plugin MUST be initialized here!
        // This prevents the "state() called before manage()" panic.
        // ----------------------------------------------------------
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        let _ = app.emit("ptt-trigger", "start");
                    } else {
                        let _ = app.emit("ptt-trigger", "stop");
                    }
                })
                .build(),
        )
        .setup(|app| {
            app.global_shortcut().register("Ctrl+Shift+Space")?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![finalize_and_type])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}