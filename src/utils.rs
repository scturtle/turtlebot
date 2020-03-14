use async_std::sync::{channel, Receiver, Sender};
use lazy_static::lazy_static;

type Message = (String, String);

lazy_static! {
    static ref CHANNEL: (Sender<Message>, Receiver<Message>) = channel(100);
}

pub async fn send(id: &str, msg: &str) {
    CHANNEL.0.send((id.to_owned(), msg.to_owned())).await
}

pub async fn recv() -> Option<(String, String)> {
    CHANNEL.1.recv().await
}

pub fn format_time(time: &chrono::NaiveDateTime) -> String {
    use chrono::offset::TimeZone;
    chrono_tz::Asia::Shanghai
        .from_utc_datetime(time)
        .format("%m-%d %H:%M")
        .to_string()
}

pub async fn sleep(n: u64) {
    async_std::task::sleep(std::time::Duration::from_secs(n)).await;
}
