-- Your SQL goes here
CREATE TABLE follow_log (
  id INTEGER PRIMARY KEY NOT NULL,
  name TEXT NOT NULL,
  action TEXT NOT NULL,
  time TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
)
