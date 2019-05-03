use reqwest::header::{self, HeaderMap, HeaderName};
use serde_json::Value;
use std::time::Duration;

pub struct Twitter {
    pub client: reqwest::r#async::Client,
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
        let client = reqwest::r#async::ClientBuilder::new()
            .default_headers(headers)
            .proxy(reqwest::Proxy::all("http://localhost:1087").unwrap())
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap();
        Twitter { client }
    }
}
