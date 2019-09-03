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
use log::info;

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    info!("start");
    db::init_db();
    let fut = futures::future::join3(
        telegram::telegram_loop(),
        follow_monitor::follow_monitor_loop(),
        rss::rss_monitor_loop(),
    );
    futures::executor::block_on(fut);
}
