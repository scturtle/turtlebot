mod db;
mod dispatcher;
mod error;
mod rss;
mod tg;
mod utils;

use log::{error, info};
use tokio::select;

async fn main_loop() {
    let mut tg = tg::Telegram::new();
    rss::register(&mut tg.dispatcher);
    loop {
        select! {
            to_send = crate::utils::recv() => {
                if let Some((id, msg)) = to_send {
                    if let Err(err) = tg.send(id, msg).await {
                        error!("tg send error: {}", err);
                    }
                } else {
                    log::error!("channel recv error");
                }
            }
            msg = tg.get() => {
                match msg {
                    Ok(j) => tg.process(j).await,
                    Err(err) => error!("tg get error: {}", err),
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    env_logger::init();
    info!("start");
    db::init(&db::get_conn()).expect("init db");
    main_loop().await;
}
