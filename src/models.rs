use crate::schema::follow_log;
use crate::schema::rss;

#[derive(Queryable, Identifiable, AsChangeset)]
#[table_name = "follow_log"]
pub struct FollowLog {
    pub id: i32,
    pub name: String,
    pub action: String,
    pub time: chrono::NaiveDateTime,
}

#[derive(Queryable, Identifiable, AsChangeset)]
#[table_name = "rss"]
pub struct Rss {
    pub id: i32,
    pub home: String,
    pub title: String,
    pub feed: String,
    pub latest_title: String,
    pub latest_link: String,
}
