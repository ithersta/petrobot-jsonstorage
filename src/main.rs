#[macro_use]
extern crate rocket;

use hex;
use serde::Serialize;
use rocket::http::Status;
use rocket::response::status;
use rocket::State;
use sha2::{Digest, Sha256};
use shuttle_runtime::CustomError;
use sqlx::{FromRow, PgPool};
use sqlx::migrate::Migrator;

struct AppState {
    pool: PgPool,
}

#[derive(Serialize, FromRow)]
struct StoredJSON {
    pub id: String,
    pub json: String,
}

#[get("/<id>")]
async fn load(id: String, state: &State<AppState>) -> Result<String, status::Custom<String>> {
    let stored_json: StoredJSON = sqlx::query_as("SELECT * FROM jsons WHERE id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => status::Custom(
                Status::NotFound,
                "the requested json does not exist".into(),
            ),
            _ => status::Custom(
                Status::InternalServerError,
                "something went wrong".into(),
            ),
        })?;

    Ok(stored_json.json)
}

#[post("/", data = "<json>")]
async fn store(json: String, state: &State<AppState>) -> Result<String, status::Custom<String>> {
    let hash = Sha256::digest(&json);
    let id = hex::encode(hash);

    sqlx::query("INSERT INTO jsons(id, json) VALUES ($1, $2)")
        .bind(&id)
        .bind(json.as_str())
        .execute(&state.pool)
        .await
        .map_err(|_| {
            status::Custom(
                Status::InternalServerError,
                "something went wrong".into(),
            )
        })?;

    Ok(id)
}

static MIGRATOR: Migrator = sqlx::migrate!();

#[shuttle_runtime::main]
async fn rocket(#[shuttle_shared_db::Postgres] pool: PgPool) -> shuttle_rocket::ShuttleRocket {
    MIGRATOR.run(&pool).await.map_err(CustomError::new)?;

    let state = AppState { pool };
    let rocket = rocket::build()
        .mount("/", routes![store, load])
        .manage(state);

    Ok(rocket.into())
}
