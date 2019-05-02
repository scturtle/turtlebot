use crate::models::FollowLog;
use diesel::prelude::*;
use futures::future::IntoFuture;
use futures::{task, try_ready, Async, Future, Poll};
use log::{error, info};
use reqwest::header::{self, HeaderMap, HeaderName};
use serde_derive::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

pub struct Twitter {
    client: reqwest::r#async::Client,
}

impl Twitter {
    pub fn new() -> Self {
        let secret = std::fs::read_to_string("secret.json").unwrap();
        let val: Value = serde_json::from_str(&secret).unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-csrf-token"),
            val["x-csrf-token"].as_str().unwrap().parse().unwrap(),
        );
        headers.insert(
            header::AUTHORIZATION,
            val["authorization"].as_str().unwrap().parse().unwrap(),
        );
        headers.insert(
            header::COOKIE,
            val["cookie"].as_str().unwrap().parse().unwrap(),
        );
        // info!("headers: {:?}", headers);
        let client = reqwest::r#async::ClientBuilder::new()
            .default_headers(headers)
            .proxy(reqwest::Proxy::all("http://localhost:1087").unwrap())
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        Twitter { client }
    }
}

pub struct FollowStatus {
    twitter: Twitter,
    url: reqwest::Url,
    cursor: i64,
    results: HashMap<String, String>,
    future: Option<Box<dyn Future<Item = Value, Error = ()> + Send>>,
}

impl FollowStatus {
    pub fn new(user_id: &str, ftype: bool) -> Self {
        let mut url = reqwest::Url::parse(if ftype {
            "https://api.twitter.com/1.1/followers/list.json"
        } else {
            "https://api.twitter.com/1.1/friends/list.json"
        })
        .unwrap();
        url.query_pairs_mut()
            .append_pair("user_id", user_id)
            .append_pair("count", "200");
        Self {
            twitter: Twitter::new(),
            url: url,
            cursor: -1,
            results: Default::default(),
            future: None,
        }
    }

    fn get(&self) -> Box<dyn Future<Item = Value, Error = ()> + Send> {
        let mut url = self.url.clone();
        url.query_pairs_mut()
            .append_pair("cursor", &self.cursor.to_string());
        Box::new(
            self.twitter
                .client
                .get(url)
                .send()
                .and_then(|mut v| v.json::<Value>())
                .map_err(|e| info!("twitter error: {}", e)),
        )
    }
}

impl Future for FollowStatus {
    type Item = HashMap<String, String>;
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, ()> {
        match &mut self.future {
            None => {
                self.future = Some(self.get());
                task::current().notify();
                return Ok(Async::NotReady);
            }
            Some(future) => {
                let j = try_ready!(future.poll());

                if !j["errors"].is_null() {
                    error!("twitter: {}", j["errors"]);
                    return Err(());
                }
                if !j["error"].is_null() {
                    error!("twitter: {}", j["error"]);
                    return Err(());
                }

                let users = j["users"].as_array().unwrap();
                for r in users {
                    self.results.insert(
                        r["id_str"].as_str().unwrap().to_owned(),
                        r["screen_name"].as_str().unwrap().to_owned(),
                    );
                }
                match j["next_cursor"].as_i64() {
                    Some(n) => {
                        if n == 0 {
                            return Ok(Async::Ready(self.results.clone()));
                        } else {
                            self.cursor = n;
                            self.future = None;
                            task::current().notify();
                            Ok(Async::NotReady)
                        }
                    }
                    _ => return Err(()),
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct FollowSnapshot {
    following: HashMap<String, String>,
    followers: HashMap<String, String>,
}

pub struct FollowMonitor {
    user_id: String,
    conn: SqliteConnection,
    snapshot: Option<FollowSnapshot>,
    follow_future: Option<
        Box<
            dyn Future<Item = (HashMap<String, String>, HashMap<String, String>), Error = ()>
                + Send,
        >,
    >,
    sleep_future: Option<tokio_timer::Delay>,
}

impl FollowMonitor {
    pub fn new() -> Self {
        use crate::schema::follow_log::dsl::*;
        let conn = crate::utils::establish_connection();
        let mut snapshot = None;
        if let Ok(log) = follow_log
            .filter(action.eq("meta"))
            .first::<FollowLog>(&conn)
        {
            snapshot = serde_json::from_str(&log.name).ok();
            info!("loaded follow snapshot {}", snapshot.is_some());
        }
        Self {
            user_id: std::env::var("TWITTER_USER_ID").unwrap(),
            conn: conn,
            snapshot: snapshot,
            follow_future: None,
            sleep_future: None,
        }
    }

    fn diff_keys<'a, V>(x: &'a HashMap<String, V>, y: &HashMap<String, V>) -> Vec<&'a str> {
        x.keys()
            .filter(|&k| !y.contains_key(k))
            .map(|k| k.as_ref())
            .collect()
    }

    fn process(
        &mut self,
        new_following: HashMap<String, String>,
        new_followers: HashMap<String, String>,
    ) {
        use crate::schema::follow_log::dsl::*;
        match &self.snapshot {
            Some(snapshot) => {
                let FollowSnapshot {
                    following,
                    followers,
                } = snapshot;
                info!("processing follow status");
                let unfo_ids = Self::diff_keys(&following, &new_following);
                let fo_ids = Self::diff_keys(&new_following, &following);
                let unfoed_ids = Self::diff_keys(&followers, &new_followers);
                let foed_ids = Self::diff_keys(&new_followers, &followers);
                info!("{} {}", new_following.len(), new_followers.len());
                info!(
                    "{} {} {} {}",
                    unfo_ids.len(),
                    fo_ids.len(),
                    unfoed_ids.len(),
                    foed_ids.len()
                );
                for uid in unfo_ids {
                    info!("unfo {}", following[uid]);
                    let _ = diesel::insert_into(follow_log)
                        .values((name.eq(&following[uid]), action.eq("unfo")))
                        .execute(&self.conn);
                }
                for uid in fo_ids {
                    info!("fo {}", new_following[uid]);
                    let _ = diesel::insert_into(follow_log)
                        .values((name.eq(&new_following[uid]), action.eq("fo")))
                        .execute(&self.conn);
                }
                for uid in unfoed_ids {
                    info!("unfoed {}", followers[uid]);
                    let _ = diesel::insert_into(follow_log)
                        .values((name.eq(&followers[uid]), action.eq("unfoed")))
                        .execute(&self.conn);
                }
                for uid in foed_ids {
                    info!("foed {}", new_followers[uid]);
                    let _ = diesel::insert_into(follow_log)
                        .values((name.eq(&new_followers[uid]), action.eq("foed")))
                        .execute(&self.conn);
                }
            }
            None => {}
        }
        self.snapshot = Some(FollowSnapshot {
            following: new_following,
            followers: new_followers,
        });
        let snapshot_str = serde_json::to_string(self.snapshot.as_ref().unwrap()).unwrap();
        match &mut follow_log
            .filter(action.eq("meta"))
            .first::<FollowLog>(&self.conn)
        {
            Ok(log) => {
                log.name = snapshot_str;
                let _ = log.save_changes::<FollowLog>(&self.conn);
            }
            _ => {
                let _ = diesel::insert_into(follow_log)
                    .values((name.eq(&snapshot_str), action.eq("meta")))
                    .execute(&self.conn);
            }
        }
        info!("follow snapshot saved");
    }
}

impl Future for FollowMonitor {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        if let Some(sleep) = &mut self.sleep_future {
            try_ready!(sleep.map_err(|_| ()).poll());
            self.sleep_future = None;
        }

        match &mut self.follow_future {
            Some(follow) => {
                let (following, followers) = try_ready!(follow.poll());
                self.follow_future = None;
                self.process(following, followers);
                self.sleep_future = Some(tokio_timer::sleep(std::time::Duration::from_secs(60)));
            }
            _ => {
                info!("fetching follow status");
                let following = FollowStatus::new(&self.user_id, false).into_future();
                let followers = FollowStatus::new(&self.user_id, true).into_future();
                self.follow_future = Some(Box::new(following.join(followers)));
            }
        }

        task::current().notify();
        Ok(Async::NotReady)
    }
}
