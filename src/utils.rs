use diesel::prelude::{Connection, SqliteConnection};
use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref SEND_QUEUE: Mutex<std::collections::VecDeque<(String, String)>> =
        Mutex::new(Default::default());
}

pub fn send(id: &str, msg: &str) {
    SEND_QUEUE
        .lock()
        .unwrap()
        .push_back((id.to_owned(), msg.to_owned()));
}

pub fn to_send() -> Option<(String, String)> {
    SEND_QUEUE.lock().unwrap().pop_front()
}

pub fn establish_connection() -> SqliteConnection {
    let database_url = std::env::var("DATABASE_URL").unwrap();
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn format_time(time: &chrono::NaiveDateTime) -> String {
    use chrono::offset::TimeZone;
    chrono_tz::Asia::Shanghai.from_utc_datetime(time).format("%m-%d %H:%M").to_string()
}
