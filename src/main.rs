#![feature(rustc_private)]
#![feature(async_await)]

mod dispatcher;
mod follow_monitor;
mod follow_status;
mod models;
mod rss;
mod schema;
mod telegram;
mod twitter;
mod utils;

#[macro_use]
extern crate diesel;
use log::info;

#[runtime::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    info!("start");
    let _ = runtime::spawn(follow_monitor::follow_monitor_loop());
    let _ = runtime::spawn(rss::rss_monitor_loop());
    telegram::telegram_loop().await;
}
