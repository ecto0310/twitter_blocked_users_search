use dotenv::dotenv;
use std::env;

mod task;
mod twitter;

fn main() {
    dotenv().ok();

    let consummer_key = env::var("CONSUMER_KEY").expect("CONSUMMER_KEY must be set.");
    let consummer_secret = env::var("CONSUMER_SECRET").expect("CONSUMMER_SECRET must be set.");
    let access_token = env::var("ACCESS_TOKEN").expect("ACCESS_TOKEN must be set.");
    let access_token_secret =
        env::var("ACCESS_TOKEN_SECRET").expect("ACCESS_TOKEN_SECRET must be set.");

    let twitter = twitter::Twitter::new(
        consummer_key,
        consummer_secret,
        access_token,
        access_token_secret,
    );

    let task = task::Task::new(twitter, 2);
}
