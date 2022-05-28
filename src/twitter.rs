pub struct Twitter {
    consumer_key: String,
    consumer_secret: String,
    access_token: String,
    access_token_secret: String,
}

impl Twitter {
    pub fn new(
        consumer_key: String,
        consumer_secret: String,
        access_token: String,
        access_token_secret: String,
    ) -> Self {
        Self {
            consumer_key,
            consumer_secret,
            access_token,
            access_token_secret,
        }
    }

    pub fn get(&self, path: &str, parameters: Vec<(&str, &str)>) -> reqwest::blocking::Response {
        let endpoint = format!("https://api.twitter.com/1.1/{}.json", path);
        let authorization_header =
            self.get_authorization_header("GET", &endpoint, parameters.clone());

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            authorization_header.parse().unwrap(),
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded".parse().unwrap(),
        );

        reqwest::blocking::Client::new()
            .get(&endpoint)
            .headers(headers)
            .body(
                parameters
                    .into_iter()
                    .map(|(key, value)| {
                        format!(
                            "{}={}",
                            percent_encoding::utf8_percent_encode(key, &Self::FRAGMENT),
                            percent_encoding::utf8_percent_encode(value, &Self::FRAGMENT)
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("&"),
            )
            .send()
            .unwrap()
    }

    const FRAGMENT: percent_encoding::AsciiSet = percent_encoding::NON_ALPHANUMERIC
        .remove(b'*')
        .remove(b'-')
        .remove(b'_')
        .remove(b'.');

    fn get_authorization_header(
        &self,
        method: &str,
        endpoint: &str,
        parameters: Vec<(&str, &str)>,
    ) -> String {
        "".to_string()
    }
}