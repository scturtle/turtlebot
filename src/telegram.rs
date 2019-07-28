use crate::dispatcher::Dispatcher;
use crate::utils::{get_async_client, to_send};
use futures::compat::Future01CompatExt;
use log::{error, info};
use serde_json::{json, Value};
use tokio::prelude::Future;

struct Telegram {
    prefix: reqwest::Url,
    client: reqwest::r#async::Client,
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
            prefix: reqwest::Url::parse(&url).unwrap(),
            client: get_async_client(),
            dispatcher: Dispatcher::new(),
            master,
            offset: 0,
        }
    }

    async fn get(&self) -> Result<Value, ()> {
        self.client
            .post(self.prefix.join("getUpdates").unwrap())
            .json(&json!({"offset": self.offset, "timeout": 60}))
            .send()
            .and_then(|mut v| v.json::<Value>())
            .map_err(|e| error!("poll error: {}", e))
            .compat()
            .await
    }

    async fn send(&self, id: String, msg: String) -> Result<(), ()> {
        self.client
            .post(self.prefix.join("sendMessage").unwrap())
            .json(
                &json!({"chat_id": &id, "text": &msg, "parse_mode": "Markdown",
                          "disable_web_page_preview": true}),
            )
            .send()
            .and_then(|mut v| v.json::<Value>())
            .map(|resp| match resp["ok"] {
                Value::Bool(true) => {}
                _ => error!("send error: {}", resp.to_string()),
            })
            .map_err(|e| error!("send error: {}", e))
            .compat()
            .await
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
            let _ = await!(tg.send(id, msg));
        }
        if let Ok(j) = await!(tg.get()) {
            tg.process(j);
        }
    }
}
