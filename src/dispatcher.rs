use crate::utils::send;
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait Callback {
    async fn callback(&self, cid: &str, msg: &str);
}

#[derive(Default)]
pub struct Dispatcher {
    callbacks: HashMap<String, Box<dyn Callback + Sync>>,
}

impl Dispatcher {
    pub fn register(&mut self, cmd: &str, callback: Box<dyn Callback + Sync>) {
        self.callbacks.insert(cmd.to_owned(), callback);
    }

    pub async fn dispatch(&self, cid: &str, msg: &str) {
        if let Some(cmd) = msg.split_whitespace().next() {
            if let Some(callback) = self.callbacks.get(cmd) {
                let callback = callback.as_ref() as &dyn Callback;
                callback.callback(cid, msg).await;
            } else {
                send(cid, "???").await;
            }
        } else {
            send(cid, "???").await;
        }
    }
}
