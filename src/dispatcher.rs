use crate::models::FollowLog;
use crate::rss;
use crate::utils::{establish_connection, format_time, send};
use diesel::prelude::*;
use log::error;

pub struct Dispatcher {}

impl Dispatcher {
    pub fn new() -> Self {
        Self {}
    }

    pub fn dispatch(&self, cid: &str, msg: &str) {
        let cols: Vec<_> = msg.split_whitespace().collect();
        if cols.get(0) == Some(&"/f") {
            send(cid, &self.cmd_f(msg));
        } else if cols.get(0) == Some(&"/rss") {
            send(cid, &rss::list());
        } else if cols.get(0) == Some(&"/sub") {
            if let Some(url) = cols.get(1) {
                send(cid, &rss::sub(url));
            } else {
                send(cid, "need url");
            }
        } else if cols.get(0) == Some(&"/unsub") {
            if let Some(id_to_del) = cols.get(1).and_then(|t| t.parse::<i32>().ok()) {
                send(cid, &rss::unsub(id_to_del));
            } else {
                send(cid, "need id to del");
            }
        } else {
            send(cid, "???");
        }
    }

    fn cmd_f(&self, _msg: &str) -> String {
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
