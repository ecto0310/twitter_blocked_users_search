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

    pub fn init(&self) {}
    pub fn run(&self) {}
    pub fn result(&self) {}
}

struct Status {
    limit_distance: i32,
}

impl Status {
    pub fn new(limit_distance: i32) -> Self {
        Self {
            limit_distance: limit_distance,
        }
    }
}
