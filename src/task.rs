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
        let res = self.twitter.get("account/verify_credentials", Vec::new());
        let res_json = res.json::<serde_json::Value>().unwrap();
        self.status.my_id = res_json["id_str"].as_str().unwrap().to_string();
        match Status::load(&self.status.my_id) {
            Ok(status) => self.status = status,
            Err(_) => {
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
        }
    }

    pub fn run(&mut self) {
        self.fetch()
    }
    pub fn result(&self) {}

    fn fetch(&mut self) {
        while 0 < self.status.fetch_queue.len() {
            let command = self.status.fetch_queue.pop_front().unwrap();
            match command {
                Fetch::Follow(data) => self.fetch_follow(data),
                Fetch::Follower(data) => self.fetch_follower(data),
            }
            self.status.save();
            std::thread::sleep(std::time::Duration::from_secs(60));
        }
    }

    fn fetch_follow(&mut self, data: FetchStatus) {
        if self.status.users[&data.id].distance != data.distance {
            return;
        }
        let mut parameters: Vec<(&str, &str)> = Vec::new();
        parameters.push(("user_id", &data.id));
        parameters.push(("cursor", &data.cursor));
        parameters.push(("stringify_ids", "true"));
        parameters.push(("count", "5000"));
        let res = self.twitter.get("friends/ids", parameters);
        if res.status() == reqwest::StatusCode::UNAUTHORIZED {
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
        let mut parameters: Vec<(&str, &str)> = Vec::new();
        parameters.push(("user_id", &data.id));
        parameters.push(("cursor", &data.cursor));
        parameters.push(("stringify_ids", "true"));
        parameters.push(("count", "5000"));
        let res = self.twitter.get("followers/ids", parameters);
        if res.status() == reqwest::StatusCode::UNAUTHORIZED {
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
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Status {
    limit_distance: i32,
    my_id: String,
    fetch_queue: std::collections::VecDeque<Fetch>,
    users: std::collections::HashMap<String, User>,
}

#[derive(serde::Serialize, serde::Deserialize)]
enum Fetch {
    Follow(FetchStatus),
    Follower(FetchStatus),
}

#[derive(serde::Serialize, serde::Deserialize)]
struct FetchStatus {
    id: String,
    cursor: String,
    distance: i32,
}

#[derive(serde::Serialize, serde::Deserialize)]
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

    fn save(&self) {
        let json = serde_json::to_string(&self).unwrap();
        let mut file = std::fs::File::create(format!("{}.json", self.my_id)).unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }

    fn load(my_id: &String) -> Result<Status, std::io::Error> {
        let file = std::fs::File::open(format!("{}.json", my_id))?;
        let json = std::io::BufReader::new(file);
        let data = serde_json::from_reader(json)?;
        Ok(data)
    }
}
