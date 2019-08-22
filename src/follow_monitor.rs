use crate::db::{get_conn, insert_follow_log, load_follow_snapshot, save_follow_snapshot};
use crate::follow_status::FollowStatus;
use crate::utils::sleep;
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
        let conn = get_conn();
        let mut snapshot = None;
        if let Ok(snapshot_str) = load_follow_snapshot(&conn) {
            snapshot = serde_json::from_str(&snapshot_str).ok();
            info!("loaded follow snapshot {}", snapshot.is_some());
        }
        Self {
            user_id: std::env::var("TWITTER_USER_ID").unwrap(),
            snapshot,
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
        let conn = get_conn();
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
                    if let Err(e) = insert_follow_log(&conn, &following[uid], "unfo") {
                        error!("insert_follow_log: {}", e);
                    }
                }
                for uid in fo_ids {
                    info!("fo {}", new_following[uid]);
                    if let Err(e) = insert_follow_log(&conn, &new_following[uid], "fo") {
                        error!("insert_follow_log: {}", e);
                    }
                }
                for uid in unfoed_ids {
                    info!("unfoed {}", followers[uid]);
                    if let Err(e) = insert_follow_log(&conn, &followers[uid], "unfoed") {
                        error!("insert_follow_log: {}", e);
                    }
                }
                for uid in foed_ids {
                    info!("foed {}", new_followers[uid]);
                    if let Err(e) = insert_follow_log(&conn, &new_followers[uid], "foed") {
                        error!("insert_follow_log: {}", e);
                    }
                }
            }
            None => {}
        }
        self.snapshot = Some(FollowSnapshot {
            following: new_following,
            followers: new_followers,
        });
        let snapshot_str = serde_json::to_string(self.snapshot.as_ref().unwrap()).unwrap();
        if let Err(e) = save_follow_snapshot(&conn, &snapshot_str) {
            error!("failed to save follow meta: {}", e);
        } else {
            info!("follow snapshot saved");
        }
    }
}

pub async fn follow_monitor_loop() {
    let mut fm = FollowMonitor::new();
    loop {
        info!("fetching follow status");
        let mut following_future = FollowStatus::new(&fm.user_id, false);
        let following = following_future.fetch().await;
        let mut followers_future = FollowStatus::new(&fm.user_id, true);
        let followers = followers_future.fetch().await;
        match (following, followers) {
            (Some(following), Some(followers)) => {
                fm.process(following, followers);
                sleep(fm.interval).await;
            }
            _ => info!("fetch follow status failed"),
        }
    }
}
