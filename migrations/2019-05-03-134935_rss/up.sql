-- Your SQL goes here
CREATE TABLE rss (
  id INTEGER PRIMARY KEY NOT NULL,
  url TEXT NOT NULL,
  title TEXT NOT NULL,
  latest_title TEXT NOT NULL,
  latest_url TEXT NOT NULL
)
