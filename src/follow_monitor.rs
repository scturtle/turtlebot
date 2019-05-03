use crate::follow_status::FollowStatus;
use crate::models::FollowLog;
use crate::utils::FutureBox;
use diesel::prelude::*;
use futures::future::IntoFuture;
use futures::{task, Async, Future, Poll};
use log::{error, info};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
struct FollowSnapshot {
    following: HashMap<String, String>,
    followers: HashMap<String, String>,
}

pub struct FollowMonitor {
    user_id: String,
    conn: SqliteConnection,
    snapshot: Option<FollowSnapshot>,
    follow_future: Option<FutureBox<(HashMap<String, String>, HashMap<String, String>)>>,
    sleep_future: Option<tokio_timer::Delay>,
}

impl FollowMonitor {
    pub fn new() -> Self {
        use crate::schema::follow_log::dsl::*;
        let conn = crate::utils::establish_connection();
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
            conn: conn,
            snapshot: snapshot,
            follow_future: None,
            sleep_future: None,
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
                        .execute(&self.conn);
                }
                for uid in fo_ids {
                    info!("fo {}", new_following[uid]);
                    let _ = diesel::insert_into(follow_log)
                        .values((name.eq(&new_following[uid]), action.eq("fo")))
                        .execute(&self.conn);
                }
                for uid in unfoed_ids {
                    info!("unfoed {}", followers[uid]);
                    let _ = diesel::insert_into(follow_log)
                        .values((name.eq(&followers[uid]), action.eq("unfoed")))
                        .execute(&self.conn);
                }
                for uid in foed_ids {
                    info!("foed {}", new_followers[uid]);
                    let _ = diesel::insert_into(follow_log)
                        .values((name.eq(&new_followers[uid]), action.eq("foed")))
                        .execute(&self.conn);
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
            .first::<FollowLog>(&self.conn)
            .optional()
        {
            Ok(Some(log)) => {
                log.name = snapshot_str;
                let _ = log.save_changes::<FollowLog>(&self.conn);
            }
            Ok(None) => {
                let _ = diesel::insert_into(follow_log)
                    .values((name.eq(&snapshot_str), action.eq("meta")))
                    .execute(&self.conn);
            }
            Err(e) => error!("{}", e),
        }
        info!("follow snapshot saved");
    }
}

impl Future for FollowMonitor {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        if let Some(sleep) = &mut self.sleep_future {
            match sleep.poll() {
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                _ => {},
            }
            self.sleep_future = None;
        }

        match &mut self.follow_future {
            Some(follow) => {
                let (following, followers) = match follow.poll() {
                    Ok(Async::Ready(res)) => res,
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(_) => {
                        self.follow_future = None;
                        task::current().notify();
                        return Ok(Async::NotReady)
                    }
                };
                self.follow_future = None;
                self.process(following, followers);
                self.sleep_future = Some(tokio_timer::sleep(std::time::Duration::from_secs(60)));
            }
            _ => {
                info!("fetching follow status");
                let following = FollowStatus::new(&self.user_id, false).into_future();
                let followers = FollowStatus::new(&self.user_id, true).into_future();
                self.follow_future = Some(Box::new(following.join(followers)));
            }
        }

        task::current().notify();
        Ok(Async::NotReady)
    }
}
