use crate::db::{delete_rss, get_conn, insert_rss, list_rss, update_rss};
use crate::dispatcher::{Callback, Dispatcher};
use crate::error::MyError;
use crate::utils::{send, sleep};
use async_trait::async_trait;
use log::{error, info};

struct List {}

#[async_trait]
impl Callback for List {
    async fn callback(&self, cid: &str, _: &str) {
        let conn = get_conn();
        let rs = list_rss(&conn).unwrap_or_else(|e| {
            error!("{}", e);
            vec![]
        });
        let reply = rs
            .into_iter()
            .map(|r| format!("{} [{}]({})", r.id, r.title, r.home))
            .collect::<Vec<_>>()
            .join("\n");
        if reply.is_empty() {
            send(cid, "no results").await;
        } else {
            send(cid, &reply).await;
        }
    }
}

fn parse_feed(feed: feed_rs::model::Feed, url_str: &str) -> (String, String, String, String) {
    (
        feed.links
            .first()
            .map(|l| l.href.clone())
            .unwrap_or(url_str.to_owned()),
        feed.title
            .map(|title| title.content)
            .unwrap_or_else(|| "no title".to_owned()),
        feed.entries
            .first()
            .and_then(|e| e.title.as_ref().map(|t| t.content.clone()))
            .unwrap_or_default(),
        feed.entries
            .first()
            .and_then(|e| e.links.first().map(|l| l.href.clone()))
            .unwrap_or_default(),
    )
}

async fn process_sub_url(url_str: &str) -> Result<(String, String, String, String), MyError> {
    let url = url::Url::parse(url_str)?;
    let bytes = reqwest::get(url.clone()).await?.bytes().await?;
    match feed_rs::parser::parse(&bytes[..]) {
        // the url is just feed url
        Ok(feed) => Ok(parse_feed(feed, url_str)),
        Err(_) => {
            // find feed url in page
            let text = String::from_utf8_lossy(&bytes);
            match feedfinder::detect_feeds(&url, &text)
                .ok()
                .and_then(|feeds| feeds.first().map(|f| f.url().clone()))
            {
                None => Err(MyError::Custom("no feed found".to_owned())),
                Some(feed_url) => {
                    // how to do recursive async fn?
                    let url = url::Url::parse(feed_url.as_ref())?;
                    let bytes = reqwest::get(url.clone()).await?.bytes().await?;
                    match feed_rs::parser::parse(&bytes[..]) {
                        Ok(feed) => Ok(parse_feed(feed, url_str)),
                        Err(_) => Err(MyError::Custom("no feed found".to_owned())),
                    }
                }
            }
        }
    }
}

struct Sub {}

#[async_trait]
impl Callback for Sub {
    async fn callback(&self, cid: &str, msg: &str) {
        let Some(url_str) = msg.split_whitespace().nth(1) else {
            send(cid, "need url").await;
            return;
        };
        let (feed_str, title_str, latest_title_str, latest_link_str) =
            match process_sub_url(url_str).await {
                Ok(res) => res,
                Err(err) => {
                    error!("{}", err);
                    send(cid, "no feed found").await;
                    return;
                }
            };
        let conn = get_conn();
        if let Err(e) = insert_rss(
            &conn,
            url_str,
            &title_str,
            &feed_str,
            &latest_title_str,
            &latest_link_str,
        ) {
            error!("{}", e);
            send(cid, "error in db").await;
        } else {
            send(cid, &format!("subcribed \"{}\"", title_str)).await;
        }
    }
}

struct Unsub {}

#[async_trait]
impl Callback for Unsub {
    async fn callback(&self, cid: &str, msg: &str) {
        let Some(id_to_del) = msg
            .split_whitespace()
            .nth(1)
            .and_then(|t| t.parse::<i32>().ok())
        else {
            send(cid, "need id to del").await;
            return;
        };
        let conn = get_conn();
        let reply = match delete_rss(&conn, id_to_del) {
            Ok(n) => {
                if n > 0 {
                    "done"
                } else {
                    "not found"
                }
            }
            Err(e) => {
                error!("{}", e);
                "error"
            }
        };
        send(cid, reply).await;
    }
}

pub fn register(dispatcher: &mut Dispatcher) {
    dispatcher.register("/rss", Box::new(List {}));
    dispatcher.register("/sub", Box::new(Sub {}));
    dispatcher.register("/unsub", Box::new(Unsub {}));
}

pub async fn rss_monitor_loop() {
    let cid = std::env::var("MASTER_ID").unwrap();
    let interval = std::env::var("RSS_INTERVAL").unwrap().parse().unwrap();
    loop {
        let conn = get_conn();
        let rs = list_rss(&conn).unwrap_or_else(|e| {
            error!("{}", e);
            vec![]
        });
        for r in rs {
            info!("fetch {}", r.feed);
            let resp = match reqwest::get(&r.feed).await {
                Err(e) => {
                    error!("{}", e);
                    continue;
                }
                Ok(resp) => resp,
            };
            let bytes = match resp.bytes().await {
                Err(e) => {
                    error!("{}", e);
                    continue;
                }
                Ok(bytes) => bytes,
            };
            let feed = match feed_rs::parser::parse(&bytes[..]) {
                Err(e) => {
                    error!("{}", e);
                    continue;
                }
                Ok(feed) => feed,
            };

            let entries: Vec<(String, String)> = feed
                .entries
                .iter()
                .map(|e| {
                    (
                        e.title
                            .as_ref()
                            .map(|t| t.content.clone())
                            .unwrap_or_default(),
                        e.links.first().map(|l| l.href.clone()).unwrap_or_default(),
                    )
                })
                .collect();

            let cnt = entries
                .iter()
                .position(|(a, b)| (a, b) == (&r.latest_title, &r.latest_link))
                .unwrap_or(1);

            let mut msg = String::new();
            for (new_title, new_link) in entries.iter().take(cnt) {
                // update with the first one
                if msg.is_empty() {
                    if let Err(e) = update_rss(&conn, r.id, new_title, new_link) {
                        error!("{}", e);
                    }
                }
                info!("new post [{}]({})", new_title, new_link);
                msg.push_str(&format!("\n[{}]({})", new_title, new_link));
            }
            if !msg.is_empty() {
                send(&cid, msg.trim_start()).await;
            }
        }
        drop(conn);
        sleep(interval).await;
    }
}
