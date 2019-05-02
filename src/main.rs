mod telegram;
use log::info;

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    let tg = telegram::Telegram::new();
    info!("start");
    tokio::run(tg);
}
