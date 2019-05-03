use crate::schema::follow_log;

#[derive(Queryable, Identifiable, AsChangeset)]
#[table_name = "follow_log"]
pub struct FollowLog {
    pub id: i32,
    pub name: String,
    pub action: String,
    pub time: chrono::NaiveDateTime,
}
