// A native window with no console and no web server. The page talks to the application through a
// custom protocol handled in this process, so no network port is opened.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use our_budget::app::App;
use our_budget::event_store::EventStore;
use our_budget::mediator::Mediator;
use our_budget::setup;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::WindowBuilder;
use wry::http::{header::CONTENT_TYPE, Method, Request, Response};
use wry::{WebContext, WebViewBuilder};

/// Statement rows before this date are not imported.
const IMPORT_FROM: &str = "2026-11-01";
const INDEX: &str = include_str!("index.html");

/// The `data` folder in the current folder, else beside the executable, else a new one in the current folder.
fn data_dir() -> PathBuf {
    let here = PathBuf::from("data");
    if here.is_dir() {
        return here;
    }
    if let Some(beside) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("data"))) {
        if beside.is_dir() {
            return beside;
        }
    }
    here
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        let dir = data_dir();
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(dir.join("error.txt"), &e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let data = data_dir();
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

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Our Budget")
        .with_inner_size(LogicalSize::new(1180.0, 820.0))
        .build(&event_loop)
        .map_err(|e| e.to_string())?;

    let mut context = WebContext::new(Some(data.join("webview")));
    let builder = WebViewBuilder::new_with_web_context(&mut context)
        .with_custom_protocol("app".into(), move |_id, request| respond(&app, &mediator, request).map(Into::into))
        .with_url("app://localhost");

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let _webview = builder.build(&window).map_err(|e| e.to_string())?;
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let _webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().ok_or("no window to draw in")?;
        builder.build_gtk(vbox).map_err(|e| e.to_string())?
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            *control_flow = ControlFlow::Exit;
        }
    });
}

/// Serves the page, and runs `POST /api/<RequestType>` through the mediator.
fn respond(app: &Mutex<App>, mediator: &Mediator, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let path = request.uri().path().to_string();
    let (status, content_type, body) = match (request.method() == Method::POST, path.strip_prefix("/api/")) {
        (true, Some(name)) => {
            let raw = String::from_utf8_lossy(request.body()).to_string();
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
        (false, _) if path == "/" => (200, "text/html; charset=utf-8", INDEX.to_string()),
        _ => (404, "text/plain", "not found".to_string()),
    };
    Response::builder().status(status).header(CONTENT_TYPE, content_type).body(body.into_bytes()).unwrap()
}
