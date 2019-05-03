use crate::dispatcher::Dispatcher;
use crate::utils::to_send;
use futures::future::Future as OldFuture;
use log::{error, info};
use serde_json::{json, Value};
use std::future::Future;
use tokio_async_await::compat::forward::IntoAwaitable;

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
            client: reqwest::r#async::ClientBuilder::new()
                .proxy(reqwest::Proxy::all("http://localhost:1087").unwrap())
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap(),
            dispatcher: Dispatcher::new(),
            master: master,
            offset: 0,
        }
    }

    fn get(&self) -> impl Future<Output = Result<Value, ()>> {
        self.client
            .post(self.prefix.join("getUpdates").unwrap())
            .json(&json!({"offset": self.offset, "timeout": 60}))
            .send()
            .and_then(|mut v| v.json::<Value>())
            .map_err(|e| error!("poll error: {}", e))
            .into_awaitable()
    }

    fn send(&self, id: String, msg: String) -> impl Future<Output = Result<(), ()>> {
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
            .map_err(|e| error!("poll error: {}", e))
            .into_awaitable()
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
