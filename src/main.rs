use our_budget::app::App;
use our_budget::event_store::EventStore;
use our_budget::mediator::Mediator;
use our_budget::setup;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const PORT: u16 = 4000;
/// Statement rows before this date are not imported.
const IMPORT_FROM: &str = "2026-11-01";
const INDEX: &str = include_str!("index.html");

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let data = PathBuf::from("data");
    let store = EventStore::open(&data.join("databases").join("our-budget.db"))?;
    let mut app = App::new(store, data.join("back-accounts"))?;
    app.import_from = IMPORT_FROM.to_string();
    println!(
        "read model tables: {} ({} events applied)",
        if app.rebuilt { "rebuilt from the events" } else { "up to date" },
        app.replayed
    );
    let mediator = Arc::new(Mediator::default());
    setup::register_accounts(&mut app, &mediator, &data.join("secrets.md"))?;
    let app = Arc::new(Mutex::new(app));

    // Import on start, then every hour.
    {
        let (app, mediator) = (app.clone(), mediator.clone());
        std::thread::spawn(move || loop {
            if let Ok(mut a) = app.lock() {
                match mediator.send(&mut a, "ImportStatements", &json!({})) {
                    Ok(r) => println!("import: {} new transactions", r["imported"]),
                    Err(e) => eprintln!("import failed: {e}"),
                }
            }
            std::thread::sleep(Duration::from_secs(3600));
        });
    }

    let server = tiny_http::Server::http(("127.0.0.1", PORT)).map_err(|e| e.to_string())?;
    println!("Our Budget running at http://localhost:{PORT}");
    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let post = *request.method() == tiny_http::Method::Post;
        let (status, content_type, body) = match (post, url.strip_prefix("/api/")) {
            (true, Some(name)) => {
                let mut raw = String::new();
                let _ = request.as_reader().read_to_string(&mut raw);
                let input: Value = if raw.trim().is_empty() { json!({}) } else { serde_json::from_str(&raw).unwrap_or(Value::Null) };
                if input.is_null() {
                    (400, "application/json", json!({ "error": "invalid JSON" }).to_string())
                } else {
                    let result = match app.lock() {
                        Ok(mut a) => mediator.send(&mut a, name, &input),
                        Err(_) => Err("server error".to_string()),
                    };
                    match result {
                        Ok(v) => (200, "application/json", v.to_string()),
                        Err(e) => (400, "application/json", json!({ "error": e }).to_string()),
                    }
                }
            }
            (false, _) if url == "/" || url.starts_with("/?") => (200, "text/html; charset=utf-8", INDEX.to_string()),
            _ => (404, "text/plain", "not found".to_string()),
        };
        let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], content_type.as_bytes()).unwrap();
        let _ = request.respond(tiny_http::Response::from_string(body).with_status_code(status).with_header(header));
    }
    Ok(())
}
