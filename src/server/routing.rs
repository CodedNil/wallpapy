use crate::{
    common::WallpaperData,
    server::{
        auth::login_server,
        commenting::{add_comment, remove_comment},
        image, DATABASE_PATH, IMAGES_TREE,
    },
};
use axum::{
    routing::{get, post},
    Router,
};
use time::OffsetDateTime;

const NEW_WALLPAPER_INTERVAL: time::Duration = time::Duration::hours(6);

pub fn setup_routes(app: Router) -> Router {
    app.route("/login", post(login_server))
        .route("/get", get(image::get))
        .route("/latest", get(image::latest))
        .route("/favourites", get(image::favourites))
        .route("/generate", post(image::generate))
        .route("/commentadd", post(add_comment))
        .route("/commentremove", post(remove_comment))
        .route("/imageliked", post(image::like))
        .route("/imageremove", post(image::remove))
        .route("/imagerecreate", post(image::recreate))
}

pub async fn start_server() {
    loop {
        match sled::open(DATABASE_PATH).and_then(|db| db.open_tree(IMAGES_TREE)) {
            Ok(images_tree) => {
                let cur_time = OffsetDateTime::now_utc();
                let latest_time = images_tree
                    .iter()
                    .values()
                    .filter_map(|v| v.ok().and_then(|bytes| bincode::deserialize(&bytes).ok()))
                    .max_by_key(|wallpaper: &WallpaperData| wallpaper.datetime)
                    .map_or(cur_time, |image| image.datetime);
                if cur_time - latest_time > NEW_WALLPAPER_INTERVAL {
                    if let Err(err) = image::generate_wallpaper_impl(None, None).await {
                        log::error!("Error generating wallpaper: {:?}", err);
                    }
                }
            }
            Err(e) => log::error!("{:?}", e),
        }

        // Sleep for an hour
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}
