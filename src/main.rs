#![feature(rustc_private)]
#![feature(await_macro, async_await)]

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
#[macro_use]
extern crate tokio; // use tokio::await;
use log::info;

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    info!("start");
    tokio::run_async(async {
        tokio::spawn_async(follow_monitor::follow_monitor_loop());
        tokio::spawn_async(rss::rss_monitor_loop());
        await!(telegram::telegram_loop());
    });
}
