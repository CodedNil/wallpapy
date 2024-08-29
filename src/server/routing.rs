use super::{
    auth::login_server,
    commenting::{add_comment, remove_comment},
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
        .route("/commentadd", post(add_comment))
        .route("/commentremove", post(remove_comment))
}
