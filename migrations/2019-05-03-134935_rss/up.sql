-- Your SQL goes here
CREATE TABLE rss (
  id INTEGER PRIMARY KEY NOT NULL,
  home TEXT NOT NULL,
  title TEXT NOT NULL,
  feed TEXT NOT NULL,
  latest_title TEXT NOT NULL,
  latest_link TEXT NOT NULL
)
