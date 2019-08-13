use crate::dispatcher::Dispatcher;
use crate::utils::to_send;
use log::{error, info};
use serde_json::{json, Value};

struct Telegram {
    prefix: String,
    dispatcher: Dispatcher,
    master: String,
    offset: i64,
}

impl Telegram {
    fn new() -> Self {
        let master = std::env::var("MASTER_ID").unwrap();
        let tg_key = std::env::var("TG_KEY").unwrap();
        let url = "https://api.telegram.org/bot".to_owned() + &tg_key + "/";
        Self {
            prefix: url.to_owned(),
            dispatcher: Dispatcher::new(),
            master,
            offset: 0,
        }
    }

    async fn get(&self) -> Result<Value, ()> {
        use isahc::prelude::*;
        Request::post(self.prefix.to_owned() + "getupdates")
            .timeout(std::time::Duration::from_secs(60))
            .header("Content-Type", "application/json")
            .body(json!({"offset": self.offset, "timeout": 60}).to_string())
            .map_err(|e| error!("body error: {}", e))?
            .send_async()
            .await
            .map_err(|e| error!("send error: {}", e))?
            .into_body()
            .json()
            .map_err(|e| error!("json error: {}", e))
    }

    async fn send(&self, id: String, msg: String) -> Result<(), ()> {
        use isahc::prelude::*;
        Request::post(self.prefix.to_owned() + "sendMessage")
            .timeout(std::time::Duration::from_secs(60))
            .header("Content-Type", "application/json")
            .body(
                json!({"chat_id": &id, "text": &msg, "parse_mode": "Markdown",
                       "disable_web_page_preview": true})
                .to_string(),
            )
            .map_err(|e| error!("body error: {}", e))?
            .send_async()
            .await
            .map_err(|e| error!("send error: {}", e))?
            .into_body()
            .json::<Value>()
            .map(|resp| match resp["ok"] {
                Value::Bool(true) => {}
                _ => error!("send error: {}", resp.to_string()),
            })
            .map_err(|e| error!("json error: {}", e))
    }

    fn process(&mut self, j: Value) {
        if Value::Bool(true) != j["ok"] {
            error!("polling error: {:?}", j["description"]);
        } else {
            for m in j["result"].as_array().unwrap() {
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
                    self.dispatcher.dispatch(&cid, text);
                }
            }
        }
    }
}

pub async fn telegram_loop() {
    let mut tg = Telegram::new();
    loop {
        if let Some((id, msg)) = to_send() {
            let _ = tg.send(id, msg).await;
        }
        if let Ok(j) = tg.get().await {
            tg.process(j);
        }
    }
}
