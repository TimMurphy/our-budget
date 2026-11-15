//! Hand-rolled mediator: one handler per command or query type.

use crate::app::App;
use crate::handlers;
use serde_json::Value;
use std::collections::HashMap;

pub type Handler = fn(&mut App, &Value) -> Result<Value, String>;

#[derive(Debug)]
pub struct Mediator {
    handlers: HashMap<&'static str, Handler>,
}

impl Default for Mediator {
    fn default() -> Self {
        let mut m = Mediator { handlers: HashMap::new() };
        for (name, h) in handlers::all() {
            m.handlers.insert(name, h);
        }
        m
    }
}

impl Mediator {
    pub fn send(&self, app: &mut App, request: &str, body: &Value) -> Result<Value, String> {
        match self.handlers.get(request) {
            Some(h) => h(app, body),
            None => Err(format!("unknown request {request}")),
        }
    }
}
