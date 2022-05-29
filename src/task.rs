use super::twitter;

pub struct Task {
    twitter: twitter::Twitter,
    status: Status,
}

impl Task {
    pub fn new(twitter: twitter::Twitter, limit_distance: i32) -> Self {
        Self {
            twitter: twitter,
            status: Status::new(limit_distance),
        }
    }

    pub fn init(&mut self) {
        let res = self.twitter.get("account/verify_credentials", Vec::new());
        let res_json = res.json::<serde_json::Value>().unwrap();
        self.status.my_id = res_json["id_str"].as_str().unwrap().to_string();
        self.status.users.insert(
            self.status.my_id.clone(),
            User {
                distance: 0,
                edge: std::collections::HashSet::new(),
            },
        );
        self.status
            .fetch_queue
            .push_back(Fetch::Follow(FetchStatus {
                id: self.status.my_id.clone(),
                cursor: "-1".to_string(),
                distance: 0,
            }));
    }
    pub fn run(&self) {}
    pub fn result(&self) {}
}

struct Status {
    limit_distance: i32,
    my_id: String,
    fetch_queue: std::collections::VecDeque<Fetch>,
    users: std::collections::HashMap<String, User>,
}

enum Fetch {
    Follow(FetchStatus),
    Follower(FetchStatus),
}

struct FetchStatus {
    id: String,
    cursor: String,
    distance: i32,
}

struct User {
    distance: i32,
    edge: std::collections::HashSet<String>,
}

impl Status {
    pub fn new(limit_distance: i32) -> Self {
        Self {
            limit_distance: limit_distance,
            my_id: String::new(),
            fetch_queue: std::collections::VecDeque::new(),
            users: std::collections::HashMap::new(),
        }
    }
}
