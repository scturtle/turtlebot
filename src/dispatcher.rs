use crate::models::FollowLog;
use crate::rss;
use crate::utils::{establish_connection, format_time, send};
use diesel::prelude::*;
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
        let conn = establish_connection();
        use crate::schema::follow_log::dsl::*;
        let s = follow_log
            .filter(action.ne("meta"))
            .order(time.desc())
            .limit(6)
            .get_results::<FollowLog>(&conn)
            .unwrap_or_else(|e| {
                error!("{}", e);
                vec![]
            })
            .into_iter()
            .map(|log| {
                format!(
                    "{0} {1} [{2}](twitter.com/{2})",
                    format_time(&log.time),
                    log.action,
                    log.name
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
