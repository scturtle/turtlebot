use async_channel::unbounded;
use async_std::{channel::Receiver, channel::Sender};
use lazy_static::lazy_static;

type Message = (String, String);

lazy_static! {
    static ref CHANNEL: (Sender<Message>, Receiver<Message>) = unbounded();
}

pub async fn send(id: &str, msg: &str) {
    if let Err(e) = CHANNEL.0.send((id.to_owned(), msg.to_owned())).await {
        log::error!("channel send error {e}");
    };
}

pub async fn recv() -> Option<(String, String)> {
    match CHANNEL.1.recv().await {
        Err(e) => {
            log::error!("channel recv error {e}");
            None
        }
        Ok(res) => Some(res),
    }
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
