use crate::db::{delete_repo, get_conn, insert_repo, list_repo, update_repo};
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
        let rs = list_repo(&conn).unwrap_or_else(|e| {
            error!("{}", e);
            vec![]
        });
        let reply = rs
            .into_iter()
            .map(|r| {
                format!(
                    "{0} [{1}](https://github.com/{1}) {2}",
                    r.id, r.name, r.latest
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        if reply.is_empty() {
            send(cid, "no results").await;
        } else {
            send(cid, &reply).await;
        }
    }
}

struct Sub {}

#[async_trait]
impl Callback for Sub {
    async fn callback(&self, cid: &str, msg: &str) {
        let Some(name) = msg.split_whitespace().nth(1) else {
            send(cid, "need repo name").await;
            return;
        };
        let latest = match get_version(name).await {
            Ok(latest) => latest,
            Err(e) => {
                send(cid, &e.to_string()).await;
                return;
            }
        };
        match insert_repo(&get_conn(), name, &latest) {
            Ok(_) => {
                send(cid, &format!("OK, latest is {}", latest)).await;
            }
            Err(e) => {
                send(cid, &e.to_string()).await;
            }
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
        let reply = match delete_repo(&conn, id_to_del) {
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

async fn get_version(name: &str) -> Result<String, MyError> {
    let url = url::Url::parse(&format!("https://github.com/{}/releases", name))?;
    let text = reqwest::get(url.clone()).await?.text().await?;
    text.split_whitespace()
        .filter_map(|l| {
            l.split_once("releases/tag/")
                .map(|x| x.1.split_once("\"").unwrap().0.to_owned())
        })
        .next()
        .ok_or(MyError::Custom("no release found".to_owned()))
}

pub fn register(dispatcher: &mut Dispatcher) {
    dispatcher.register("/repo", Box::new(List {}));
    dispatcher.register("/rsub", Box::new(Sub {}));
    dispatcher.register("/runsub", Box::new(Unsub {}));
}

pub async fn repo_monitor_loop() {
    let cid = std::env::var("MASTER_ID").unwrap();
    let interval = std::env::var("REPO_INTERVAL").unwrap().parse().unwrap();
    loop {
        let conn = get_conn();
        let rs = list_repo(&conn).unwrap_or_else(|e| {
            error!("{}", e);
            vec![]
        });
        for r in rs {
            info!("fetch repo {}", r.name);
            let latest = match get_version(&r.name).await {
                Err(e) => {
                    error!("{}", e);
                    continue;
                }
                Ok(resp) => resp,
            };
            if latest != r.latest {
                send(&cid, &format!("[{0}]({0}) {1}", r.name, r.latest)).await;
                if let Err(e) = update_repo(&conn, r.id, &latest) {
                    error!("{}", e);
                }
            }
        }
        drop(conn);
        sleep(interval).await;
    }
}
