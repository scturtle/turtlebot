use crate::utils::get_async_client_with_headers;
use reqwest::header::{self, HeaderMap, HeaderName};
use serde_json::Value;

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
        let client = get_async_client_with_headers(headers);
        Twitter { client }
    }
}
