use rusqlite::{params, Connection, Result, Row};

pub fn get_conn() -> Connection {
    Connection::open("data.db").unwrap()
}

pub fn init() -> Result<usize> {
    let conn = get_conn();
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
}

pub struct Rss {
    pub id: i32,
    pub home: String,
    pub title: String,
    pub feed: String,
    pub latest_title: String,
    pub latest_link: String,
}

impl TryFrom<&Row<'_>> for Rss {
    type Error = rusqlite::Error;
    fn try_from(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            home: row.get("home")?,
            title: row.get("title")?,
            feed: row.get("feed")?,
            latest_title: row.get("latest_title")?,
            latest_link: row.get("latest_link")?,
        })
    }
}

pub fn list_rss(conn: &Connection) -> Result<Vec<Rss>> {
    let mut stmt = conn.prepare(
        "SELECT id, home, title, feed, latest_title, latest_link from rss order by id asc",
    )?;
    let res = stmt.query_map(rusqlite::params![], |r| Rss::try_from(r))?;
    res.into_iter().collect()
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
