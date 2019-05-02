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
