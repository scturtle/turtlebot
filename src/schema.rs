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
        url -> Text,
        title -> Text,
        latest_title -> Text,
        latest_url -> Text,
    }
}

allow_tables_to_appear_in_same_query!(follow_log, rss,);
