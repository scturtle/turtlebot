use crate::db::{get_conn, get_follow_log};
use crate::rss;
use crate::utils::{format_time, send};
use log::error;
use std::ops::Deref;

pub struct Dispatcher {}

impl Dispatcher {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn dispatch(&self, cid: &str, msg: &str) {
        let cols: Vec<_> = msg.split_whitespace().collect();
        match cols.get(0).map(Deref::deref) {
            Some("/f") => {
                let n = cols.get(1).and_then(|n| n.parse().ok()).unwrap_or(6);
                send(cid, &self.cmd_f(n)).await;
            }
            Some("/rss") => send(cid, &rss::list()).await,
            Some("/sub") => {
                let url = cols.get(1).map(Deref::deref);
                let resp = url.map(rss::sub).unwrap_or("need url".into());
                send(cid, &resp).await;
            }
            Some("/unsub") => {
                let id_to_del = cols.get(1).and_then(|t| t.parse::<i32>().ok());
                let resp = id_to_del.map(rss::unsub).unwrap_or("need id to del".into());
                send(cid, &resp).await;
            }
            _ => send(cid, "???").await,
        }
    }

    fn cmd_f(&self, n: u32) -> String {
        let n = n.min(30);
        let conn = get_conn();
        let logs = match get_follow_log(&conn, n) {
            Err(e) => {
                error!("{}", e);
                return format!("{}", e);
            }
            Ok(logs) => logs,
        };
        let s = logs
            .into_iter()
            .map(|(name, action, time)| {
                format!(
                    "{0} {1} [{2}](twitter.com/{2})",
                    format_time(&time),
                    action,
                    name
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        if s.is_empty() {
            "no results".to_owned()
        } else {
            s
        }
    }
}
