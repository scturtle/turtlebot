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
extern crate tokio;
use futures::{FutureExt, TryFutureExt};
use log::info;

async fn main_loop() {
    let fut = follow_monitor::follow_monitor_loop()
        .boxed()
        .unit_error()
        .compat();
    tokio::spawn(fut);
    tokio::spawn(rss::rss_monitor_loop().boxed().unit_error().compat());
    telegram::telegram_loop().await;
}

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();
    info!("start");
    tokio::run(main_loop().boxed().unit_error().compat());
}
