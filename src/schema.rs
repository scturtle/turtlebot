table! {
    follow_log (id) {
        id -> Integer,
        name -> Text,
        action -> Text,
        time -> Timestamp,
    }
}

table! {
    rss (id) {
        id -> Integer,
        home -> Text,
        title -> Text,
        feed -> Text,
        latest_title -> Text,
        latest_link -> Text,
    }
}

allow_tables_to_appear_in_same_query!(follow_log, rss,);
