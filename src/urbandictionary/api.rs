use reqwest::{StatusCode, Url};

use super::models::{Definition, Response};

async fn search<S: AsRef<str>>(http_client: reqwest::Client, term: S) -> reqwest::Result<String> {
    let response = http_client
        .head(
            Url::parse_with_params(
                "https://www.urbandictionary.com/define.php",
                [("term", term.as_ref())],
            )
            .unwrap(),
        )
        .send()
        .await?
        .error_for_status()?;

    if response.status() == StatusCode::FOUND {
        let location = Url::parse("https://www.urbandictionary.com/")
            .unwrap()
            .join(response.headers()["Location"].to_str().unwrap())
            .unwrap();
        let term = location
            .query_pairs()
            .find(|(k, _)| k == "term")
            .unwrap()
            .1
            .into_owned();

        Ok(term)
    } else {
        Ok(term.as_ref().to_string())
    }
}

pub async fn define<S: AsRef<str>>(
    http_client: reqwest::Client,
    term: S,
) -> reqwest::Result<Option<Definition>> {
    let term = search(http_client.clone(), term).await?;
    let mut definitions = http_client
        .get(
            Url::parse_with_params(
                "https://api.urbandictionary.com/v0/define",
                [("term", term)],
            )
            .unwrap(),
        )
        .send()
        .await?
        .error_for_status()?
        .json::<Response>()
        .await?
        .list;

    if definitions.is_empty() {
        Ok(None)
    } else {
        Ok(Some(definitions.swap_remove(0)))
    }
}
