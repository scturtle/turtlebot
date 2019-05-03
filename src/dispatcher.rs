use crate::models::FollowLog;
use crate::utils::{format_time, send};
use diesel::prelude::*;
use log::error;

pub struct Dispatcher {
    conn: SqliteConnection,
}

impl Dispatcher {
    pub fn new() -> Self {
        let conn = crate::utils::establish_connection();
        Self { conn: conn }
    }

    pub fn dispatch(&self, cid: &str, msg: &str) {
        if msg == "/f" {
            send(cid, &self.cmd_f(msg));
        } else {
            send(cid, "unknown command");
        }
    }

    fn cmd_f(&self, _msg: &str) -> String {
        use crate::schema::follow_log::dsl::*;
        let s = follow_log
            .filter(action.ne("meta"))
            .order(time.desc())
            .limit(6)
            .get_results::<FollowLog>(&self.conn)
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
