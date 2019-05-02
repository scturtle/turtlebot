#![feature(rustc_private)]

mod models;
mod schema;
mod telegram;
mod twitter;
mod utils;

use log::info;
#[macro_use]
extern crate diesel;

use futures::future::{join_all, Future, IntoFuture};

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    info!("start");
    let tasks: Vec<Box<dyn Future<Item = (), Error = ()> + Send>> = vec![
        Box::new(telegram::Telegram::new().into_future()),
        Box::new(twitter::FollowMonitor::new().into_future()),
    ];
    tokio::run(join_all(tasks).map(|_| ()));
}
