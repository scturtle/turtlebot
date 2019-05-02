use futures::{task, try_ready, Async, Future, Poll};
use log::{error, info};
use reqwest::header::{self, HeaderMap, HeaderName};
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
