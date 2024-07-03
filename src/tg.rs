use crate::dispatcher::Dispatcher;
use crate::error::MyError;
use log::{error, info};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

pub struct Telegram {
    prefix: String,
    pub dispatcher: Dispatcher,
    master: String,
    offset: i64,
}

impl Telegram {
    pub fn new() -> Self {
        let master = std::env::var("MASTER_ID").unwrap();
        let tg_key = std::env::var("TG_KEY").unwrap();
        let url = format!("https://api.telegram.org/bot{}/", tg_key);
        Self {
            prefix: url,
            dispatcher: Default::default(),
            master,
            offset: 0,
        }
    }

    pub async fn get(&self) -> Result<Value, MyError> {
        Client::new()
            .post(self.prefix.to_owned() + "getupdates")
            .timeout(Duration::from_secs(60))
            .header("Content-Type", "application/json")
            .json(&json!({"offset": self.offset, "timeout": 60}))
            .send()
            .await?
            .json()
            .await
            .map_err(MyError::Request)
    }

    pub async fn send(&self, id: String, msg: String) -> Result<(), MyError> {
        let body = json!({
            "chat_id": id,
            "text": msg,
            "parse_mode": "Markdown",
            "disable_web_page_preview": true
        });
        let resp = Client::new()
            .post(self.prefix.to_owned() + "sendMessage")
            .timeout(std::time::Duration::from_secs(60))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?
            .json::<Value>()
            .await?;
        match resp["ok"] {
            Value::Bool(true) => Ok(()),
            _ => Err(MyError::Custom(resp.to_string())),
        }
    }

    pub async fn process(&mut self, json: Value) {
        if !json["ok"].as_bool().unwrap_or(false) {
            error!("polling error: {:?}", json["description"]);
        }
        for m in json["result"].as_array().unwrap_or(&vec![]) {
            let new_offset = m["update_id"].as_i64().unwrap_or(0);
            self.offset = i64::max(self.offset, new_offset + 1);
            if !m["inline_query"].is_null() || !m["chosen_inline_result"].is_null() {
                continue;
            }
            let m = match m["message"]
                .as_object()
                .or_else(|| m["edited_message"].as_object())
            {
                Some(m) => m,
                _ => continue,
            };
            let cid = match &m["chat"]["id"] {
                Value::Number(cid) => cid.to_string(),
                _ => continue,
            };
            if cid != self.master {
                continue;
            }
            if let Value::String(text) = &m["text"] {
                info!("tg recv {}", text);
                self.dispatcher.dispatch(&cid, text).await;
            }
        }
    }
}
