use crate::models::Rss;
use crate::utils::{establish_connection, send, sleep};
use diesel::prelude::*;
use futures::future::Future;
use futures::stream::Stream;
use log::{error, info};
use rss::Channel;
use std::str::FromStr;

pub fn sub(url_str: &str) -> String {
    use crate::schema::rss::dsl::*;
    // FIXME: blocking
    match &mut reqwest::get(url_str) {
        Err(_) => {
            return format!("cannot access {}", url_str);
        }
        Ok(resp) => {
            let text = resp.text().unwrap_or_default();
            let (title_str, latest_title_str, latest_url_str) =
                if let Some(triple) = parse_rss_or_atom(&text) {
                    triple
                } else {
                    return format!("cannot parse {}", url_str);
                };
            let conn = establish_connection();
            let _ = diesel::insert_into(rss)
                .values((
                    url.eq(url_str),
                    title.eq(&title_str),
                    latest_title.eq(latest_title_str),
                    latest_url.eq(latest_url_str),
                ))
                .execute(&conn);
            return format!("subcribed \"{}\"", title_str);
        }
    }
}

pub fn unsub(id_to_del: i32) -> String {
    use crate::schema::rss::dsl::*;
    let conn = establish_connection();
    match diesel::delete(rss.filter(id.eq(id_to_del))).execute(&conn) {
        Ok(n) if n > 0 => return "done".into(),
        Ok(_) => return "not found".into(),
        Err(e) => {
            error!("{}", e);
            return "error".into();
        }
    }
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
        .map(|r| format!("{} [{}]({})", r.id, r.title, r.url))
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
    let conn = establish_connection();
    let client = reqwest::r#async::ClientBuilder::new()
        .proxy(reqwest::Proxy::all("http://localhost:1087").unwrap())
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap();
    let cid = std::env::var("MASTER_ID").unwrap();
    let interval = std::env::var("FOLLOW_INTERVAL").unwrap().parse().unwrap();
    loop {
        let rs = rss
            .order(id.asc())
            .get_results::<Rss>(&conn)
            .unwrap_or_else(|e| {
                error!("{}", e);
                vec![]
            });
        for r in rs {
            info!("fetch {}", r.url);
            let url_to_get = reqwest::Url::parse(&r.url).unwrap();
            let resp = await!(client
                .get(url_to_get)
                .send()
                .and_then(|t| t.into_body().concat2())
                .into_awaitable());
            match resp {
                Err(e) => error!("{}", e),
                Ok(body) => {
                    let text = std::str::from_utf8(&body).unwrap_or_default();
                    match parse_rss_or_atom(text) {
                        None => error!("failed to parse {}", r.url),
                        Some((_, newest_title, newest_url)) => {
                            if let Ok(mut t) = rss.filter(url.eq(&r.url)).first::<Rss>(&conn) {
                                if t.latest_url != newest_url && t.latest_title != newest_title {
                                    info!("new post [{}]({})", newest_title, newest_url);
                                    send(&cid, &format!("[{}]({})", newest_title, newest_url));
                                    t.latest_url = newest_url;
                                    t.latest_title = newest_title;
                                    let _ = t.save_changes::<Rss>(&conn);
                                    error!("updated {}", r.url);
                                }
                            }
                        }
                    }
                }
            }
        }
        await!(sleep(interval));
    }
}
