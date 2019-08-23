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

    pub fn dispatch(&self, cid: &str, msg: &str) {
        let cols: Vec<_> = msg.split_whitespace().collect();
        match cols.get(0).map(Deref::deref) {
            Some("/f") => send(cid, &self.cmd_f()),
            Some("/rss") => send(cid, &rss::list()),
            Some("/sub") => {
                let url = cols.get(1).map(Deref::deref);
                let resp = url.map(rss::sub).unwrap_or("need url".into());
                send(cid, &resp);
            }
            Some("/unsub") => {
                let id_to_del = cols.get(1).and_then(|t| t.parse::<i32>().ok());
                let resp = id_to_del.map(rss::unsub).unwrap_or("need id to del".into());
                send(cid, &resp);
            }
            _ => send(cid, "???"),
        }
    }

    fn cmd_f(&self) -> String {
        let conn = get_conn();
        let logs = match get_follow_log(&conn) {
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
