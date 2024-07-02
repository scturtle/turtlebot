use crate::dispatcher::{Callback, Dispatcher};
use async_trait::async_trait;

struct List {}

#[async_trait]
impl Callback for List {
    async fn callback(&self, cid: &str, msg: &str) {
        todo!()
    }
}

struct Sub {}

#[async_trait]
impl Callback for Sub {
    async fn callback(&self, cid: &str, msg: &str) {
        todo!()
    }
}

struct Unsub {}

#[async_trait]
impl Callback for Unsub {
    async fn callback(&self, cid: &str, msg: &str) {
        todo!()
    }
}

pub fn register(dispatcher: &mut Dispatcher) {
    dispatcher.register("/list", Box::new(List {}));
    dispatcher.register("/sub", Box::new(Sub {}));
    dispatcher.register("/unsub", Box::new(Unsub {}));
}
