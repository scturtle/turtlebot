use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::OnceCell;

type Message = (String, String);

static CHANNEL: OnceCell<Arc<Mutex<(UnboundedSender<Message>, UnboundedReceiver<Message>)>>> =
    OnceCell::const_new();

async fn init_channel() -> Arc<Mutex<(UnboundedSender<Message>, UnboundedReceiver<Message>)>> {
    Arc::new(Mutex::new(mpsc::unbounded_channel()))
}

pub async fn send(id: &str, msg: &str) {
    let channel = CHANNEL.get_or_init(init_channel).await.lock().unwrap();
    if let Err(e) = channel.0.send((id.to_owned(), msg.to_owned())) {
        log::error!("channel send error {e}");
    };
}

pub async fn recv() -> Option<(String, String)> {
    let mut channel = CHANNEL.get_or_init(init_channel).await.lock().unwrap();
    channel.1.recv().await
}

pub async fn sleep(n: u64) {
    tokio::time::sleep(Duration::from_secs(n)).await;
}
