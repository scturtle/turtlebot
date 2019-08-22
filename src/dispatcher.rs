use crate::db::get_conn;
use crate::rss;
use crate::utils::{format_time, send};
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
        let mut stmt = conn
            .prepare("select name, action, time from follow_log order by time desc limit 6")
            .unwrap();
        let iter = stmt
            .query_map(rusqlite::NO_PARAMS, |r| {
                let name: String = r.get(0).unwrap();
                let action: String = r.get(1).unwrap();
                let time = r
                    .get(2)
                    .map(|t: chrono::NaiveDateTime| format_time(&t))
                    .unwrap();
                Ok(format!(
                    "{0} {1} [{2}](twitter.com/{2})",
                    time, action, name
                ))
            })
            .unwrap();
        let s = iter.filter_map(Result::ok).collect::<Vec<_>>().join("\n");
        if s.is_empty() {
            "no results".to_owned()
        } else {
            s
        }
    }
}
