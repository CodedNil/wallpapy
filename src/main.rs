mod common;
mod server_functions;
mod web;

#[cfg(feature = "server")]
mod database;
#[cfg(feature = "server")]
mod gpt;
#[cfg(feature = "server")]
mod image;
#[cfg(feature = "server")]
mod server;

fn main() {
    #[cfg(feature = "web")]
    dioxus::launch(web::app);

    #[cfg(feature = "server")]
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async move { server_run().await });
}

#[cfg(feature = "server")]
async fn server_run() {
    use axum::routing::get;
    use dioxus::{
        prelude::dioxus_server::{FullstackState, ServeConfig},
        server::DioxusRouterExt,
    };
    use tower_http::{compression::CompressionLayer, services::ServeDir};

    #[cfg(debug_assertions)]
    dotenvy::dotenv().ok();

    if !tracing::dispatcher::has_been_set() {
        dioxus::logger::init(tracing::Level::INFO).unwrap();
    }

    database::init().await.unwrap();
    std::fs::create_dir_all(&*database::WALLPAPERS_DIR).unwrap();
    tokio::spawn(server::start_server());

    let app = axum::Router::<FullstackState>::new()
        .nest_service("/wallpapers", ServeDir::new(&*database::WALLPAPERS_DIR))
        .route("/latest", get(crate::image::latest))
        .route("/favourites", get(crate::image::favourites))
        .route("/smartget", get(crate::image::smartget))
        .serve_dioxus_application(ServeConfig::new(), web::app)
        .layer(CompressionLayer::new());

    let addr = dioxus::cli_config::fullstack_address_or_localhost();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
