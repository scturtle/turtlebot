use crate::follow_status::FollowStatus;
use crate::models::FollowLog;
use crate::utils::{establish_connection, sleep};
use diesel::prelude::*;
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
struct FollowSnapshot {
    following: HashMap<String, String>,
    followers: HashMap<String, String>,
}

struct FollowMonitor {
    user_id: String,
    snapshot: Option<FollowSnapshot>,
    interval: u64,
}

impl FollowMonitor {
    fn new() -> Self {
        use crate::schema::follow_log::dsl::*;
        let conn = establish_connection();
        let mut snapshot = None;
        if let Ok(log) = follow_log
            .filter(action.eq("meta"))
            .first::<FollowLog>(&conn)
        {
            snapshot = serde_json::from_str(&log.name).ok();
            info!("loaded follow snapshot {}", snapshot.is_some());
        }
        Self {
            user_id: std::env::var("TWITTER_USER_ID").unwrap(),
            snapshot: snapshot,
            interval: std::env::var("FOLLOW_INTERVAL").unwrap().parse().unwrap(),
        }
    }

    fn diff_keys<'a, V>(x: &'a HashMap<String, V>, y: &HashMap<String, V>) -> Vec<&'a str> {
        x.keys()
            .filter(|&k| !y.contains_key(k))
            .map(|k| k.as_ref())
            .collect()
    }

    fn process(
        &mut self,
        new_following: HashMap<String, String>,
        new_followers: HashMap<String, String>,
    ) {
        use crate::schema::follow_log::dsl::*;
        let conn = establish_connection();
        match &self.snapshot {
            Some(snapshot) => {
                let FollowSnapshot {
                    following,
                    followers,
                } = snapshot;
                info!("processing follow status");
                let unfo_ids = Self::diff_keys(&following, &new_following);
                let fo_ids = Self::diff_keys(&new_following, &following);
                let unfoed_ids = Self::diff_keys(&followers, &new_followers);
                let foed_ids = Self::diff_keys(&new_followers, &followers);
                for uid in unfo_ids {
                    info!("unfo {}", following[uid]);
                    let _ = diesel::insert_into(follow_log)
                        .values((name.eq(&following[uid]), action.eq("unfo")))
                        .execute(&conn);
                }
                for uid in fo_ids {
                    info!("fo {}", new_following[uid]);
                    let _ = diesel::insert_into(follow_log)
                        .values((name.eq(&new_following[uid]), action.eq("fo")))
                        .execute(&conn);
                }
                for uid in unfoed_ids {
                    info!("unfoed {}", followers[uid]);
                    let _ = diesel::insert_into(follow_log)
                        .values((name.eq(&followers[uid]), action.eq("unfoed")))
                        .execute(&conn);
                }
                for uid in foed_ids {
                    info!("foed {}", new_followers[uid]);
                    let _ = diesel::insert_into(follow_log)
                        .values((name.eq(&new_followers[uid]), action.eq("foed")))
                        .execute(&conn);
                }
            }
            None => {}
        }
        self.snapshot = Some(FollowSnapshot {
            following: new_following,
            followers: new_followers,
        });
        let snapshot_str = serde_json::to_string(self.snapshot.as_ref().unwrap()).unwrap();
        match &mut follow_log
            .filter(action.eq("meta"))
            .first::<FollowLog>(&conn)
            .optional()
        {
            Ok(Some(log)) => {
                log.name = snapshot_str;
                let _ = log.save_changes::<FollowLog>(&conn);
            }
            Ok(None) => {
                let _ = diesel::insert_into(follow_log)
                    .values((name.eq(&snapshot_str), action.eq("meta")))
                    .execute(&conn);
            }
            Err(e) => error!("{}", e),
        }
        info!("follow snapshot saved");
    }
}

pub async fn follow_monitor_loop() {
    let mut fm = FollowMonitor::new();
    loop {
        info!("fetching follow status");
        let mut following_future = FollowStatus::new(&fm.user_id, false);
        let following = await!(following_future.fetch());
        let mut followers_future = FollowStatus::new(&fm.user_id, true);
        let followers = await!(followers_future.fetch());
        match (following, followers) {
            (Some(following), Some(followers)) => {
                fm.process(following, followers);
                await!(sleep(fm.interval));
            }
            _ => info!("fetch follow status failed"),
        }
    }
}
