mod db;
mod dispatcher;
mod follow_monitor;
mod follow_status;
mod rss;
mod telegram;
mod twitter;
mod utils;

#[macro_use]
extern crate rusqlite;
use async_std::task;
use log::info;

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    info!("start");
    db::init_db();
    task::spawn(follow_monitor::follow_monitor_loop());
    task::spawn(rss::rss_monitor_loop());
    task::block_on(telegram::telegram_loop());
}
