mod telegram;
mod utils;

use log::info;

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    info!("start");
    let tg = telegram::Telegram::new();
    tokio::run(tg);
}
