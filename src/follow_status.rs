use crate::twitter::Twitter;
use futures::compat::Future01CompatExt;
use futures01::future::Future;
use log::{error, info};
use serde_json::Value;
use std::collections::HashMap;

pub struct FollowStatus {
    twitter: Twitter,
    url: reqwest::Url,
    cursor: i64,
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
            url,
            cursor: -1,
        }
    }

    async fn get(&self) -> Result<Value, ()> {
        let mut url = self.url.clone();
        url.query_pairs_mut()
            .append_pair("cursor", &self.cursor.to_string());
        self.twitter
            .client
            .get(url)
            .send()
            .and_then(|mut v| v.json::<Value>())
            .map_err(|e| info!("twitter error: {}", e))
            .compat()
            .await
    }

    pub async fn fetch(&mut self) -> Option<HashMap<String, String>> {
        let mut results: HashMap<String, String> = Default::default();
        loop {
            match self.get().await {
                Err(_) => return None,
                Ok(j) => {
                    if !j["errors"].is_null() {
                        error!("twitter: {}", j["errors"]);
                        return None;
                    }
                    if !j["error"].is_null() {
                        error!("twitter: {}", j["error"]);
                        return None;
                    }
                    let users = j["users"].as_array().unwrap();
                    for r in users {
                        results.insert(
                            r["id_str"].as_str().unwrap().to_owned(),
                            r["screen_name"].as_str().unwrap().to_owned(),
                        );
                    }
                    match j["next_cursor"].as_i64() {
                        Some(n) => {
                            if n == 0 {
                                return Some(results);
                            } else {
                                self.cursor = n;
                            }
                        }
                        _ => return None,
                    }
                }
            }
        }
    }
}
