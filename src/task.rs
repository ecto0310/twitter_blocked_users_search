use super::twitter;
use std::io::Write;

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
        log::info!("Start initialize phase");
        let res = self.twitter.get("account/verify_credentials", Vec::new());
        let res_json = res.json::<serde_json::Value>().unwrap();
        self.status.my_id = res_json["id_str"].as_str().unwrap().to_string();
        log::info!("Get user information on {}", &self.status.my_id);
        match Status::load(&self.status.my_id) {
            Ok(status) => {
                log::info!("Success to load file ");
                self.status = status
            }
            Err(_) => {
                log::warn!("Failure to load file");
                self.status.users.insert(
                    self.status.my_id.clone(),
                    User {
                        screen_name: String::new(),
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
        }
        log::info!("Finish initialize phase");
    }

    pub fn run(&mut self) {
        self.fetch();
        if self.status.checked_users.len() == 0 {
            self.status.remaining_users = self
                .status
                .users
                .clone()
                .into_keys()
                .collect::<Vec<String>>();
        }
        self.check();
    }

    pub fn result(&self) {
        println!(
            "Blocked users graph({} users)",
            self.status.blocked_users.len()
        );
        for user in &self.status.blocked_users {
            self.result_view(user, 0);
        }
    }

    fn fetch(&mut self) {
        log::info!("Start to fetch phase");
        while 0 < self.status.fetch_queue.len() {
            log::info!("Fetch queue size: {}", self.status.fetch_queue.len());
            let command = self.status.fetch_queue.pop_front().unwrap();
            match command {
                Fetch::Follow(data) => self.fetch_follow(data),
                Fetch::Follower(data) => self.fetch_follower(data),
            }
            self.status.save();
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
        log::info!("Finish to fetch phase");
    }

    fn check(&mut self) {
        log::info!("Start to check phase");
        while 0 < self.status.remaining_users.len() {
            self.check_users();
            self.status.save();
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        log::info!("Finish to check phase");
    }

    fn fetch_follow(&mut self, data: FetchStatus) {
        if self.status.users[&data.id].distance != data.distance {
            return;
        }
        log::info!(
            "Fetching follow (distance:{}, id:{}, cursor:{})",
            data.distance,
            data.id,
            data.cursor
        );
        let mut parameters: Vec<(&str, &str)> = Vec::new();
        parameters.push(("user_id", &data.id));
        parameters.push(("cursor", &data.cursor));
        parameters.push(("stringify_ids", "true"));
        parameters.push(("count", "5000"));
        let res = self.twitter.get("friends/ids", parameters);
        if res.status() == reqwest::StatusCode::UNAUTHORIZED {
            log::warn!(
                "Cannot access user information (distance:{}, id:{})",
                data.distance,
                data.id
            );
            return;
        }
        let mut res_json = res.json::<serde_json::Value>().unwrap();
        let next = res_json["next_cursor_str"].as_str().unwrap();
        if next == "0" {
            self.status
                .fetch_queue
                .push_front(Fetch::Follower(FetchStatus {
                    id: data.id.clone(),
                    cursor: String::from("-1"),
                    distance: data.distance,
                }));
        } else {
            self.status
                .fetch_queue
                .push_front(Fetch::Follow(FetchStatus {
                    id: data.id.clone(),
                    cursor: next.to_string(),
                    distance: data.distance,
                }));
        }
        for user in res_json["ids"].as_array_mut().unwrap() {
            let id = &user.as_str().unwrap().to_string();
            if !self.status.users.contains_key(id) {
                self.status.users.insert(
                    id.clone(),
                    User {
                        screen_name: String::new(),
                        distance: data.distance + 1,
                        edge: std::collections::HashSet::new(),
                    },
                );
                if data.distance + 1 < self.status.limit_distance {
                    self.status
                        .fetch_queue
                        .push_back(Fetch::Follow(FetchStatus {
                            id: id.clone(),
                            cursor: String::from("-1"),
                            distance: data.distance + 1,
                        }));
                }
            }
            if data.distance < self.status.users[id].distance {
                self.status
                    .users
                    .get_mut(id)
                    .unwrap()
                    .edge
                    .insert(data.id.clone());
            }
        }
    }

    fn fetch_follower(&mut self, data: FetchStatus) {
        log::info!(
            "Fetching follower (distance:{}, id:{}, cursor:{})",
            data.distance,
            data.id,
            data.cursor
        );
        let mut parameters: Vec<(&str, &str)> = Vec::new();
        parameters.push(("user_id", &data.id));
        parameters.push(("cursor", &data.cursor));
        parameters.push(("stringify_ids", "true"));
        parameters.push(("count", "5000"));
        let res = self.twitter.get("followers/ids", parameters);
        if res.status() == reqwest::StatusCode::UNAUTHORIZED {
            log::warn!(
                "Cannot access user information (distance:{}, id:{})",
                data.distance,
                data.id
            );
            return;
        }
        let mut res_json = res.json::<serde_json::Value>().unwrap();
        let next = res_json["next_cursor_str"].as_str().unwrap();
        if next != "0" {
            self.status
                .fetch_queue
                .push_front(Fetch::Follower(FetchStatus {
                    id: data.id.clone(),
                    cursor: next.to_string(),
                    distance: data.distance,
                }));
        }
        for user in res_json["ids"].as_array_mut().unwrap() {
            let id = &user.as_str().unwrap().to_string();
            if !self.status.users.contains_key(id) {
                self.status.users.insert(
                    id.clone(),
                    User {
                        screen_name: String::new(),
                        distance: data.distance + 1,
                        edge: std::collections::HashSet::new(),
                    },
                );
                if data.distance + 1 < self.status.limit_distance {
                    self.status
                        .fetch_queue
                        .push_back(Fetch::Follow(FetchStatus {
                            id: id.clone(),
                            cursor: String::from("-1"),
                            distance: data.distance + 1,
                        }));
                }
            }
            if data.distance < self.status.users[id].distance {
                self.status
                    .users
                    .get_mut(id)
                    .unwrap()
                    .edge
                    .insert(data.id.clone());
            }
        }
    }

    fn check_users(&mut self) {
        log::info!(
            "Checking user (remaining count:{})",
            self.status.remaining_users.len()
        );
        let mut params: Vec<(&str, &str)> = Vec::new();
        let request_length = std::cmp::min(100, self.status.remaining_users.len());
        let users: Vec<String> = self
            .status
            .remaining_users
            .drain(0..request_length)
            .collect();
        let users_string = users.join(",");
        params.push(("user_id", &users_string));
        params.push(("include_blocked_by", "true"));
        let res = self.twitter.get("users/lookup", params);
        let mut res_json = res.json::<serde_json::Value>().unwrap();
        for user in res_json.as_array_mut().unwrap() {
            let id = user["id_str"].as_str().unwrap().to_string();
            if user["blocked_by"].as_bool().unwrap() {
                self.status.blocked_users.push(id.clone());
            }
            (*self.status.users.get_mut(&id).unwrap()).screen_name =
                user["screen_name"].as_str().unwrap().to_string();
            self.status.checked_users.push(id.clone());
        }
    }

    fn result_view(&self, id: &String, depth: i32) {
        println!(
            "{}{} @{}({}) ",
            "  ".repeat(depth as usize),
            if depth == 0 { "-" } else { "L" },
            self.status.users[id].screen_name,
            id
        );
        for user in &self.status.users[id].edge {
            self.result_view(user, depth + 1);
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Status {
    limit_distance: i32,
    my_id: String,
    fetch_queue: std::collections::VecDeque<Fetch>,
    users: std::collections::HashMap<String, User>,
    remaining_users: Vec<String>,
    checked_users: Vec<String>,
    blocked_users: Vec<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum Fetch {
    Follow(FetchStatus),
    Follower(FetchStatus),
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct FetchStatus {
    id: String,
    cursor: String,
    distance: i32,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct User {
    screen_name: String,
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
            remaining_users: Vec::new(),
            checked_users: Vec::new(),
            blocked_users: Vec::new(),
        }
    }

    fn save(&self) {
        log::info!("Saving file");
        let json = serde_json::to_string(&self).unwrap();
        let mut file = std::fs::File::create(format!("{}.json", self.my_id)).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    fn load(my_id: &String) -> Result<Status, std::io::Error> {
        log::info!("Loading file");
        let file = std::fs::File::open(format!("{}.json", my_id))?;
        let json = std::io::BufReader::new(file);
        let data = serde_json::from_reader(json)?;
        Ok(data)
    }
}
