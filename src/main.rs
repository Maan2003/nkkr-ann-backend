use actix_web::{
    error::ErrorServiceUnavailable,
    get,
    web::{Data, Json, Query},
    App, HttpResponse, HttpServer, Result,
};
use serde::{Deserialize, Serialize};
use sqlx::{migrate, query, query_scalar};
use std::env;

type Db = sqlx::Pool<sqlx::Postgres>;

#[derive(Serialize)]
struct Notif {
    link: String,
    title: String,
    id: i32,
}

#[derive(Serialize)]
struct Notifs {
    notifs: Vec<Notif>,
    total: u64,
}

#[derive(Deserialize, Debug)]
struct Pagination {
    offset: Option<u64>,
}

fn db_error<E>(_: E) -> actix_web::Error {
    ErrorServiceUnavailable("DB error")
}

#[get("/notifs")]
async fn notifs(db: Data<Db>, pg: Query<Pagination>) -> Result<HttpResponse> {
    let total = query_scalar!("SELECT COUNT(*) AS cnt FROM notifs")
        .fetch_one(db.get_ref())
        .await
        .map_err(db_error)?
        .unwrap_or(0) as u64;

    let notifs = query!(
        "SELECT id, title, link
             FROM notifs 
             ORDER BY id DESC
             OFFSET $1
             LIMIT 20",
        pg.offset.unwrap_or(0) as i64,
    )
    .fetch_all(db.get_ref())
    .await
    .map_err(db_error)?;

    let notifs = notifs
        .into_iter()
        .map(|x| Notif {
            link: x.link,
            title: x.title,
            id: x.id,
        })
        .collect();

    let body = Notifs { notifs, total };
    Ok(HttpResponse::Ok()
        .insert_header(("Charset", "UTF-8"))
        .json(body))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    // Get the port number to listen on.
    let port = env::var("PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .expect("PORT must be a number");

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let db = Db::connect(&database_url).await.unwrap();
    migrate!().run(&db).await.unwrap();

    HttpServer::new(move || App::new().service(notifs).data(db.clone()))
        .bind(("0.0.0.0", port))?
        .run()
        .await
}
