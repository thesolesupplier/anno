mod services;
mod utils;

use axum::body::Body;
use axum::http::header::{ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE, ORIGIN};
use axum::{routing::post, Json, Router};
use dotenv::dotenv;
use hyper::Request;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tower_http::{compression::CompressionLayer, cors::CorsLayer, trace::TraceLayer};

use services::{chat_gpt, Git};
use utils::error::AppError;

// #[derive(Deserialize, Debug, Clone)]
// struct RequestData {
//     hook_id: u32,
// }

#[derive(Serialize, Debug, Clone)]
struct ResponseData {
    summary: String,
}

async fn post_deployment(// Json(payload): Json<RequestData>,
) -> Result<(StatusCode, Json<ResponseData>), AppError> {
    let repo_path = "constantincerdan/photography-website.git";

    let diff = Git::new(repo_path)?.get_diff_with_head("229d67b")?;
    let summary = chat_gpt::get_diff_summary(&diff).await?;

    Ok((StatusCode::OK, Json(ResponseData { summary })))
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .json()
        .init();

    let trace_layer =
        TraceLayer::new_for_http().on_request(|_: &Request<Body>, _: &tracing::Span| {
            tracing::info!(message = "begin request!")
        });

    let cors_layer = CorsLayer::new()
        .allow_headers([ACCEPT, ACCEPT_ENCODING, AUTHORIZATION, CONTENT_TYPE, ORIGIN])
        .allow_methods(tower_http::cors::Any)
        .allow_origin(tower_http::cors::Any);

    let app = Router::new()
        .route("/deployment", post(post_deployment))
        .layer(cors_layer)
        .layer(trace_layer)
        .layer(CompressionLayer::new().gzip(true).deflate(true));

    #[cfg(debug_assertions)]
    {
        let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
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
