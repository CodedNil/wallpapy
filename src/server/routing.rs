use crate::server::{auth::login_server, commenting, format_duration, image, read_database};
use axum::{
    Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use bincode::serde::encode_to_vec;
use chrono::{Duration, Utc};
use log::{error, info};

const NEW_WALLPAPER_INTERVAL: Duration = Duration::hours(6);

pub fn setup_routes(app: Router) -> Router {
    app.route("/login", post(login_server))
        .route("/get", get(get_database))
        .route("/latest", get(image::latest))
        .route("/favourites", get(image::favourites))
        .route("/smartget", get(image::smartget))
        .route("/generate", post(image::generate))
        .route("/commentadd", post(commenting::add))
        .route("/commentremove", post(commenting::remove))
        .route("/imageliked", post(image::like))
        .route("/imageremove", post(image::remove))
        .route("/imagerecreate", post(image::recreate))
        .route("/styles", post(commenting::styles))
        .route("/queryprompt", post(commenting::query_prompt))
}

pub async fn get_database() -> impl IntoResponse {
    let database = match read_database().await {
        Ok(database) => database,
        Err(e) => {
            error!("{e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match encode_to_vec(&database, bincode::config::standard()) {
        Ok(data) => (StatusCode::OK, data).into_response(),
        Err(e) => {
            error!("{e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

pub async fn start_server() {
    loop {
        match read_database().await {
            Ok(database) => {
                let cur_time = Utc::now();
                let latest_time = database
                    .wallpapers
                    .values()
                    .max_by_key(|wallpaper| wallpaper.datetime)
                    .map_or(cur_time, |wallpaper| wallpaper.datetime);

                info!(
                    "Time since last wallpaper: {}",
                    format_duration(cur_time - latest_time)
                );

                if cur_time - latest_time > NEW_WALLPAPER_INTERVAL
                    && let Err(e) = image::generate_wallpaper_impl(None, None).await
                {
                    error!("Error generating wallpaper: {e:?}");
                }
            }
            Err(e) => error!("{e:?}"),
        }

        // Sleep for 10 minutes
        tokio::time::sleep(tokio::time::Duration::from_secs(60 * 10)).await;
    }
}
