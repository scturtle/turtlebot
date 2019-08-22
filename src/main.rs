mod db;
mod dispatcher;
mod follow_monitor;
mod follow_status;
mod rss;
mod telegram;
mod twitter;
mod utils;

use log::info;

#[runtime::main]
async fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    info!("start");
    db::init_db();
    let _ = runtime::spawn(follow_monitor::follow_monitor_loop());
    let _ = runtime::spawn(rss::rss_monitor_loop());
    telegram::telegram_loop().await;
}
