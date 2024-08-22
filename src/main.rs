#![allow(clippy::too_many_lines)]

mod common;

#[cfg(feature = "gui")]
mod client;

#[cfg(not(target_arch = "wasm32"))]
mod server;

pub static PORT: u16 = 4560;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    simple_logger::SimpleLogger::new()
        .with_level(log::LevelFilter::Info)
        .init()
        .unwrap();

    // Set up router
    let app = server::routing::setup_routes(
        axum::Router::new()
            .nest_service("/", tower_http::services::ServeDir::new("dist"))
            .nest_service(
                "/wallpapers",
                tower_http::services::ServeDir::new("wallpapers"),
            )
            .layer(tower_http::compression::CompressionLayer::new()),
    );

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], PORT));
    println!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    #[cfg(not(feature = "gui"))]
    axum::serve(listener, app).await.unwrap();

    #[cfg(feature = "gui")]
    {
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([400.0, 300.0])
                .with_min_inner_size([300.0, 220.0])
                .with_icon(
                    eframe::icon_data::from_png_bytes(
                        &include_bytes!("../assets/icon-256.png")[..],
                    )
                    .unwrap(),
                ),
            ..Default::default()
        };
        let _ = eframe::run_native(
            "Wallpapy",
            native_options,
            Box::new(|cc| Ok(Box::new(client::app::Wallpapy::new(cc)))),
        );
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    eframe::WebLogger::init(log::LevelFilter::Info).ok(); // Redirect `log` message to `console.log`

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "wallpapy_canvas",
                web_options,
                Box::new(|cc| Ok(Box::new(client::app::Wallpapy::new(cc)))),
            )
            .await
            .expect("failed to start eframe");
    });
}
