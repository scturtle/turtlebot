use isahc::prelude::*;
use log::error;
use serde_json::Value;

pub struct Twitter {
    cfg: Value,
}

impl Twitter {
    pub fn new() -> Self {
        let secret = std::fs::read_to_string("secret.json").unwrap();
        let cfg: Value = serde_json::from_str(&secret).unwrap();
        Twitter { cfg }
    }
    pub async fn send(&self, url: url::Url) -> Result<Value, ()> {
        let client = isahc::HttpClient::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap();
        let request = isahc::http::Request::builder()
            .header("x-csrf-token", self.cfg["x-csrf-token"].as_str().unwrap())
            .header("authorization", self.cfg["authorization"].as_str().unwrap())
            .header("cookie", self.cfg["cookie"].as_str().unwrap())
            .uri(String::from(url));
        client
            .send_async(request.body(()).unwrap())
            .await
            .map_err(|e| error!("twitter error: {}", e))?
            .json()
            .await
            .map_err(|e| error!("json error: {}", e))
    }
}
