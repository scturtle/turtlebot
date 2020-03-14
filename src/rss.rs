use crate::db::{delete_rss, get_conn, insert_rss, list_rss, update_rss};
use crate::utils::{send, sleep};
use feedfinder::detect_feeds;
use isahc::prelude::*;
use log::{error, info};
use rss::Channel;
use std::str::FromStr;

pub fn sub(url_str: &str) -> String {
    // FIXME: blocking
    let url = match url::Url::parse(url_str) {
        Ok(url) => url,
        _ => return "not url".into(),
    };
    let text = match isahc::get(url_str) {
        Ok(mut resp) => resp.text().unwrap_or_default(),
        _ => return format!("cannot access {}", url_str),
    };
    let (feed_str, title_str, (latest_title_str, latest_link_str)) = match parse_rss_or_atom(&text)
    {
        // if is feed link already
        Some((title_str, articles)) => (
            url_str.into(),
            title_str,
            articles.first().cloned().unwrap_or_default(),
        ),
        // search feed link in it
        None => match detect_feeds(&url, &text)
            .ok()
            .and_then(|feeds| feeds.get(0).map(|f| f.url().clone()))
        {
            None => return format!("no feed found in {}", url_str),
            Some(feed_url) => {
                let feed_str = feed_url.to_string();
                match isahc::get(feed_url.to_string()) {
                    Err(_) => return format!("cannot access {}", feed_str),
                    Ok(mut resp) => {
                        let feed_text = resp.text().unwrap_or_default();
                        match parse_rss_or_atom(&feed_text) {
                            None => return format!("cannot parse {}", feed_str),
                            Some((title_str, articles)) => (
                                feed_str,
                                title_str,
                                articles.first().cloned().unwrap_or_default(),
                            ),
                        }
                    }
                }
            }
        },
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
    }
    return format!("subcribed \"{}\"", title_str);
}

pub fn unsub(id_to_del: i32) -> String {
    let conn = get_conn();
    match delete_rss(&conn, id_to_del) {
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
    }
    .into()
}

pub fn list() -> String {
    let conn = get_conn();
    let rs = list_rss(&conn).unwrap_or_else(|e| {
        error!("{}", e);
        vec![]
    });
    let s = rs
        .into_iter()
        .map(|r| format!("{} [{}]({})", r.id, r.title, r.home))
        .collect::<Vec<_>>()
        .join("\n");
    if s.is_empty() {
        "no results".into()
    } else {
        s
    }
}

fn parse_rss_or_atom(text: &str) -> Option<(String, Vec<(String, String)>)> {
    if let Ok(ch) = Channel::from_str(&text) {
        let title_str = ch.title().to_owned();
        let articles = ch
            .items()
            .iter()
            .map(|item| {
                (
                    item.title().map(String::from).unwrap_or_default(),
                    item.link().map(String::from).unwrap_or_default(),
                )
            })
            .collect();
        Some((title_str, articles))
    } else if let Ok(feed) = atom_syndication::Feed::from_str(&text) {
        let title_str = feed.title().to_owned();
        let articles = feed
            .entries()
            .iter()
            .map(|entry| {
                (
                    entry.title().into(),
                    entry
                        .links()
                        .first()
                        .map(|l| l.href().into())
                        .unwrap_or_default(),
                )
            })
            .collect();
        Some((title_str, articles))
    } else {
        None
    }
}

pub async fn rss_monitor_loop() {
    let cid = std::env::var("MASTER_ID").unwrap();
    let interval = std::env::var("FOLLOW_INTERVAL").unwrap().parse().unwrap();
    let conn = get_conn();
    loop {
        let rs = list_rss(&conn).unwrap_or_else(|e| {
            error!("{}", e);
            vec![]
        });
        for r in rs {
            info!("fetch {}", r.feed);
            let text = match isahc::get_async(&r.feed).await {
                Ok(mut resp) => resp.text().unwrap_or_default(),
                Err(e) => {
                    error!("{}", e);
                    continue;
                }
            };
            let result = parse_rss_or_atom(&text);
            if result.is_none() {
                error!("failed to parse {}", r.feed);
                continue;
            }
            let (_, articles) = result.unwrap();
            let cnt = articles
                .iter()
                .position(|(a, b)| (a, b) == (&r.latest_title, &r.latest_link))
                .unwrap_or(1);
            let mut msg = String::new();
            for (new_title, new_link) in articles.iter().take(cnt) {
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
                send(&cid, &msg[1..]).await;
            }
        }
        sleep(interval).await;
    }
}
