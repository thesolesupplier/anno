mod ai;
mod middleware;
mod routes;
mod services;
mod utils;

use axum::http::header::{ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE, ORIGIN};
use axum::{routing::post, Router};
use std::str::FromStr;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};
use tracing::Level;
use utils::config;

#[tokio::main]
async fn main() {
    config::load();

    let log_level = config::get("LOG_LEVEL").unwrap();

    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(Level::from_str(&log_level).unwrap())
        .json()
        .init();

    let cors_layer = CorsLayer::new()
        .allow_headers([ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE, ORIGIN])
        .allow_methods(tower_http::cors::Any)
        .allow_origin(tower_http::cors::Any);

    let app = Router::new()
        .route("/pull-request", post(routes::pull_request::post))
        .route("/workflow", post(routes::workflow::post))
        .layer(cors_layer)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new().gzip(true).deflate(true));

    // If compiled in debug mode, run the app as a regular Axum server.
    #[cfg(debug_assertions)]
    {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
        println!("App listening at http://localhost:3000");
        axum::serve(listener, app).await.unwrap();
    }

    // If compiled in release mode, run the app using the Lambda runtime.
    #[cfg(not(debug_assertions))]
    {
        let app = tower::ServiceBuilder::new()
            .layer(axum_aws_lambda::LambdaLayer::default())
            .service(app);

        lambda_http::run(app).await.unwrap();
    }
}
