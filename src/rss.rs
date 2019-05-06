use crate::models::Rss;
use crate::utils::{establish_connection, get_async_client, send, sleep};
use diesel::prelude::*;
use feedfinder::detect_feeds;
use futures::future::Future;
use futures::stream::Stream;
use log::{error, info};
use rss::Channel;
use std::str::FromStr;

pub fn sub(url_str: &str) -> String {
    use crate::schema::rss::dsl::*;
    // FIXME: blocking
    let url = match url::Url::parse(url_str) {
        Ok(url) => url,
        _ => return "not url".into(),
    };
    let text = match reqwest::get(url.clone()) {
        Ok(mut resp) => resp.text().unwrap_or_default(),
        _ => return format!("cannot access {}", url_str),
    };
    let (feed_str, (title_str, latest_title_str, latest_link_str)) = match parse_rss_or_atom(&text)
    {
        Some(triple) => (url_str.into(), triple),
        None => match detect_feeds(&url, &text)
            .ok()
            .and_then(|feeds| feeds.get(0).map(|f| f.url().clone()))
        {
            None => return format!("no feed found in {}", url_str),
            Some(feed_url) => {
                let feed_str = feed_url.to_string();
                match reqwest::get(feed_url) {
                    Err(_) => return format!("cannot access {}", feed_str),
                    Ok(mut resp) => {
                        let feed_text = resp.text().unwrap_or_default();
                        match parse_rss_or_atom(&feed_text) {
                            None => return format!("cannot parse {}", feed_str),
                            Some(triple) => (feed_str, triple),
                        }
                    }
                }
            }
        },
    };
    let conn = establish_connection();
    let _ = diesel::insert_into(rss)
        .values((
            home.eq(url_str),
            title.eq(&title_str),
            feed.eq(feed_str),
            latest_title.eq(latest_title_str),
            latest_link.eq(latest_link_str),
        ))
        .execute(&conn);
    return format!("subcribed \"{}\"", title_str);
}

pub fn unsub(id_to_del: i32) -> String {
    use crate::schema::rss::dsl::*;
    let conn = establish_connection();
    match diesel::delete(rss.filter(id.eq(id_to_del))).execute(&conn) {
        Ok(n) if n > 0 => "done",
        Ok(_) => "not found",
        Err(e) => {
            error!("{}", e);
            "error"
        }
    }
    .into()
}

pub fn list() -> String {
    use crate::schema::rss::dsl::*;
    let conn = establish_connection();
    let s = rss
        .order(id.asc())
        .get_results::<Rss>(&conn)
        .unwrap_or_else(|e| {
            error!("{}", e);
            vec![]
        })
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

fn parse_rss_or_atom(text: &str) -> Option<(String, String, String)> {
    if let Ok(ch) = Channel::from_str(&text) {
        let title_str = ch.title().to_owned();
        if let Some(item) = ch.items().first() {
            Some((
                title_str,
                item.title().map(String::from).unwrap_or_default(),
                item.link().map(String::from).unwrap_or_default(),
            ))
        } else {
            Some((title_str, String::new(), String::new()))
        }
    } else if let Ok(feed) = atom_syndication::Feed::from_str(&text) {
        let title_str = feed.title().to_owned();
        if let Some(entry) = feed.entries().first() {
            Some((
                title_str,
                entry.title().into(),
                entry
                    .links()
                    .first()
                    .map(|l| l.href().into())
                    .unwrap_or_default(),
            ))
        } else {
            Some((title_str, String::new(), String::new()))
        }
    } else {
        None
    }
}

pub async fn rss_monitor_loop() {
    use crate::schema::rss::dsl::*;
    let cid = std::env::var("MASTER_ID").unwrap();
    let interval = std::env::var("FOLLOW_INTERVAL").unwrap().parse().unwrap();
    let client = get_async_client();
    let conn = establish_connection();
    loop {
        let rs = rss
            .order(id.asc())
            .get_results::<Rss>(&conn)
            .unwrap_or_else(|e| {
                error!("{}", e);
                vec![]
            });
        for r in rs {
            info!("fetch {}", r.feed);
            let url_to_get = reqwest::Url::parse(&r.feed).unwrap();
            let resp = await!(client
                .get(url_to_get)
                .send()
                .and_then(|t| t.into_body().concat2())
                .into_awaitable());
            if let Err(e) = resp {
                error!("{}", e);
                continue;
            }
            let body = resp.unwrap();
            let text = std::str::from_utf8(&body).unwrap_or_default();
            let triple = parse_rss_or_atom(text);
            if triple.is_none() {
                error!("failed to parse {}", r.feed);
                continue;
            }
            let (_, newest_title, newest_link) = triple.unwrap();
            if let Ok(mut t) = rss.filter(feed.eq(&r.feed)).first::<Rss>(&conn) {
                if t.latest_link != newest_link && t.latest_title != newest_title {
                    info!("new post [{}]({})", newest_title, newest_link);
                    send(&cid, &format!("[{}]({})", newest_title, newest_link));
                    t.latest_link = newest_link;
                    t.latest_title = newest_title;
                    let _ = t.save_changes::<Rss>(&conn);
                    error!("updated {}", r.feed);
                }
            }
        }
        await!(sleep(interval));
    }
}
