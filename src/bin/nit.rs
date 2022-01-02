use std::{env, error::Error};

use reqwest as req;
use scraper::{Html, Selector};
use sqlx::query;

const NIT_URL: &str = "https://nitkkr.ac.in";
const SELECTORS: &str = "#main-content div.container div.col-md-4 marquee a";

type Db = sqlx::Pool<sqlx::Postgres>;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let db = Db::connect(&database_url).await.unwrap();

    let resp = req::get(NIT_URL).await?.text().await?;
    let resp = Html::parse_document(&resp);
    let selector = Selector::parse(SELECTORS).unwrap();

    let mut v = Vec::new();
    // send the body in embed
    for notif in resp.select(&selector) {
        let text = notif.text().collect::<Vec<_>>().join(" ");
        // not found a single alphanumberic character
        if !text.chars().any(|x| x.is_ascii_alphanumeric()) {
            continue;
        }

        let link = notif.value().attr("href").unwrap();
        let url = url::Url::parse(link).unwrap();
        let url = url.as_str();
        v.push((url.to_string(), text));
        // found a already existing notification, abort
    }
    for (url, text) in v.into_iter().rev() {
        if query!(
            "SELECT id FROM notifs WHERE title = $1 AND link = $2",
            text,
            url
        )
        .fetch_optional(&db)
        .await?
        .is_some()
        {
            continue;
        }

        query!(
            "INSERT INTO notifs (title, link) VALUES ($1, $2)",
            text,
            url
        )
        .execute(&db)
        .await?;
    }
    Ok(())
}
