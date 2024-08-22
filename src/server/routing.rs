use super::{
    auth::login_server,
    image::{generate_wallpaper, get_wallpapers},
};
use axum::{
    routing::{get, post},
    Router,
};

pub fn setup_routes(app: Router) -> Router {
    app.route("/login", post(login_server))
        .route("/get", get(get_wallpapers))
        .route("/generate", post(generate_wallpaper))
}
