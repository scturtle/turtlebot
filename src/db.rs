use rusqlite::{Connection, Result};

pub fn get_conn() -> Connection {
    Connection::open("data.db").unwrap()
}

pub fn init_db() {
    let conn = get_conn();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS follow_log (
  id INTEGER PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  action TEXT NOT NULL,
  time TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL)",
        params![],
    )
    .unwrap();
    conn.execute(
        "CREATE TABLE IF NOT EXISTS rss (
  id INTEGER PRIMARY KEY NOT NULL,
  home TEXT NOT NULL,
  title TEXT NOT NULL,
  feed TEXT NOT NULL,
  latest_title TEXT NOT NULL,
  latest_link TEXT NOT NULL)",
        params![],
    )
    .unwrap();
}

pub fn load_follow_snapshot(conn: &Connection) -> Result<String> {
    conn.query_row(
        "SELECT name FROM follow_log where action = \"meta\"",
        params![],
        |r| r.get(0),
    )
}

pub fn save_follow_snapshot(conn: &Connection, snapshot: &str) -> Result<usize> {
    let mut stmt = conn.prepare("SELECT name FROM follow_log where action = \"meta\"")?;
    if stmt.exists(params![]).unwrap_or(false) {
        conn.execute(
            "UPDATE follow_log set name = ?1 where action = ?2",
            params![snapshot, "meta"],
        )
    } else {
        conn.execute(
            "INSERT INTO follow_log (name, action) VALUES (?1, ?2)",
            params![snapshot, "meta"],
        )
    }
}

pub fn insert_follow_log(conn: &Connection, name: &str, action: &str) -> Result<usize> {
    conn.execute(
        "INSERT INTO follow_log (name, action) VALUES (?1, ?2)",
        params![name, action],
    )
}

pub fn get_follow_log(
    conn: &Connection,
    n: u32,
) -> Result<Vec<(String, String, chrono::NaiveDateTime)>> {
    let mut stmt =
        conn.prepare("select name, action, time from follow_log where action != \"meta\" order by time desc limit ?1")?;
    let v = stmt
        .query_map(params![n], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .filter_map(Result::ok)
        .collect();
    Ok(v)
}

pub struct Rss {
    pub id: i32,
    pub home: String,
    pub title: String,
    pub feed: String,
    pub latest_title: String,
    pub latest_link: String,
}

pub fn list_rss(conn: &Connection) -> Result<Vec<Rss>> {
    let mut stmt = conn.prepare(
        "SELECT id, home, title, feed, latest_title, latest_link from rss order by id asc",
    )?;
    let v = stmt
        .query_map(rusqlite::params![], |r| {
            Ok(Rss {
                id: r.get(0)?,
                home: r.get(1)?,
                title: r.get(2)?,
                feed: r.get(3)?,
                latest_title: r.get(4)?,
                latest_link: r.get(5)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(v)
}

pub fn insert_rss(
    conn: &Connection,
    home: &str,
    title: &str,
    feed: &str,
    latest_title: &str,
    latest_link: &str,
) -> Result<usize> {
    conn.execute(
        "INSERT INTO rss (home, title, feed, latest_title, latest_link) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![home, title, feed, latest_title, latest_link])
}

pub fn delete_rss(conn: &Connection, id_to_del: i32) -> Result<usize> {
    conn.execute("DELETE FROM rss where id = ?1", params![id_to_del])
}

pub fn update_rss(
    conn: &Connection,
    id: i32,
    latest_title: &str,
    latest_link: &str,
) -> Result<usize> {
    conn.execute(
        "UPDATE rss set latest_title = ?1, latest_link = ?2 where id = ?3",
        params![latest_title, latest_link, id],
    )
}
