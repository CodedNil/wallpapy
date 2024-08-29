use crate::server::{
    auth::login_server,
    commenting::{add_comment, remove_comment},
    image,
};
use axum::{
    routing::{get, post},
    Router,
};

pub fn setup_routes(app: Router) -> Router {
    app.route("/login", post(login_server))
        .route("/get", get(image::get))
        .route("/generate", post(image::generate))
        .route("/commentadd", post(add_comment))
        .route("/commentremove", post(remove_comment))
        .route("/imageliked", post(image::like))
        .route("/imageremove", post(image::remove))
}
