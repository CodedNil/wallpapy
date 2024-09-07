use crate::server::{auth::login_server, commenting, image, read_database};
use axum::{
    routing::{get, post},
    Router,
};
use time::{Duration, OffsetDateTime};

const NEW_WALLPAPER_INTERVAL: time::Duration = time::Duration::hours(6);

pub fn setup_routes(app: Router) -> Router {
    app.route("/login", post(login_server))
        .route("/get", get(image::get))
        .route("/latest", get(image::latest))
        .route("/favourites", get(image::favourites))
        .route("/smartget", get(image::smartget))
        .route("/generate", post(image::generate))
        .route("/commentadd", post(commenting::add))
        .route("/commentremove", post(commenting::remove))
        .route("/imageliked", post(image::like))
        .route("/imageremove", post(image::remove))
        .route("/imagerecreate", post(image::recreate))
        .route("/keystyle", post(commenting::key_style))
}

pub async fn start_server() {
    loop {
        match read_database().await {
            Ok(database) => {
                // Generate a new wallpaper every NEW_WALLPAPER_INTERVAL
                let cur_time = OffsetDateTime::now_utc();
                let latest_time = database
                    .wallpapers
                    .iter()
                    .max_by_key(|(_, wallpaper)| wallpaper.datetime)
                    .map_or(cur_time, |(_, wallpaper)| wallpaper.datetime);
                log::info!(
                    "Time since last wallpaper: {}",
                    format_duration(cur_time - latest_time)
                );
                if cur_time - latest_time > NEW_WALLPAPER_INTERVAL {
                    if let Err(err) = image::generate_wallpaper_impl(None, None).await {
                        log::error!("Error generating wallpaper: {:?}", err);
                    }
                }
            }
            Err(e) => log::error!("{:?}", e),
        }

        // Sleep for 10 minutes
        tokio::time::sleep(tokio::time::Duration::from_secs(60 * 10)).await;
    }
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.whole_seconds();
    if total_seconds < 60 {
        return "less than a minute".to_string();
    }

    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;

    match (hours, minutes) {
        (0, m) => format!("{} minute{}", m, if m == 1 { "" } else { "s" }),
        (h, 0) => format!("{} hour{}", h, if h == 1 { "" } else { "s" }),
        (h, m) => format!(
            "{} hour{} {} minute{}",
            h,
            if h == 1 { "" } else { "s" },
            m,
            if m == 1 { "" } else { "s" }
        ),
    }
}
