use crate::dispatcher::Dispatcher;
use crate::utils::{to_send, FutureBox};
use futures::{task, Async, Future, Poll};
use log::{error, info};
use serde_json::{json, Value};

pub struct Telegram {
    prefix: reqwest::Url,
    client: reqwest::r#async::Client,
    dispatcher: Dispatcher,
    master: String,
    offset: i64,
    get_future: Option<FutureBox<Value>>,
    send_future: Option<FutureBox<()>>,
}

impl Telegram {
    pub fn new() -> Self {
        let master = std::env::var("MASTER").unwrap();
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
            get_future: None,
            send_future: None,
        }
    }

    fn get(&self) -> FutureBox<Value> {
        Box::new(
            self.client
                .post(self.prefix.join("getUpdates").unwrap())
                .json(&json!({"offset": self.offset, "timeout": 60}))
                .send()
                .and_then(|mut v| v.json::<Value>())
                // .map(|v| { info!("{}", v); v })
                .map_err(|e| error!("poll error: {}", e)),
        )
    }

    fn send(&self, id: &str, msg: &str) -> FutureBox<()> {
        Box::new(
            self.client
                .post(self.prefix.join("sendMessage").unwrap())
                .json(
                    &json!({"chat_id": id, "text": msg, "parse_mode": "Markdown",
                          "disable_web_page_preview": true}),
                )
                .send()
                .and_then(|mut v| v.json::<Value>())
                .map(|resp| match resp["ok"] {
                    Value::Bool(true) => {}
                    _ => error!("send error: {}", resp.to_string()),
                })
                .map_err(|e| error!("poll error: {}", e)),
        )
    }
}

impl Future for Telegram {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        match &mut self.send_future {
            None => {
                if let Some((id, msg)) = to_send() {
                    self.send_future = Some(self.send(&id, &msg));
                    task::current().notify();
                    return Ok(Async::NotReady);
                }
            }
            Some(send) => {
                match send.poll() {
                    Ok(Async::Ready(_)) => {}
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(_) => error!("send error"),
                }
                self.send_future = None;
            }
        }

        let j: Value = match &mut self.get_future {
            None => {
                self.get_future = Some(self.get());
                task::current().notify();
                return Ok(Async::NotReady);
            }
            Some(get) => match get.poll() {
                Ok(Async::Ready(j)) => {
                    self.get_future = None;
                    j
                }
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(_) => {
                    self.get_future = None;
                    error!("send error");
                    task::current().notify();
                    return Ok(Async::NotReady);
                }
            },
        };

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

        task::current().notify();
        Ok(Async::NotReady)
    }
}
