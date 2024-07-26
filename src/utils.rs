use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::Mutex;

type Message = (String, String);

static CHANNEL: LazyLock<(Sender<Message>, Arc<Mutex<Receiver<Message>>>)> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel(8);
    (tx, Arc::new(Mutex::new(rx)))
});

pub async fn send(id: &str, msg: &str) {
    if let Err(e) = CHANNEL.0.send((id.to_owned(), msg.to_owned())).await {
        log::error!("channel send error {e}");
    };
}

pub async fn recv() -> Option<(String, String)> {
    CHANNEL.1.lock().await.recv().await
}

pub async fn sleep(n: u64) {
    tokio::time::sleep(std::time::Duration::from_secs(n)).await;
}
