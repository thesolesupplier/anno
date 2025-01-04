use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
    http::HeaderValue,
};
use hmac_sha256::HMAC;
use hyper::StatusCode;
use serde::de::DeserializeOwned;
use shared::utils::config;
use subtle::ConstantTimeEq;

pub struct GithubEvent<T>(pub T);

impl<T, S> FromRequest<S> for GithubEvent<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: Request, _: &S) -> Result<Self, Self::Rejection> {
        let validate = config::get("WEBHOOK_VALIDATION") == "true";

        let (parts, body) = req.into_parts();

        let body_as_bytes = convert_body_to_bytes(body).await?;

        if validate {
            let token = config::get("GITHUB_WEBHOOK_SECRET");
            let signature = parts.headers.get("X-Hub-Signature-256");

            validate_body(signature, &body_as_bytes, token)?;
        }

        let value = deseralise_body(body_as_bytes)?;

        Ok(GithubEvent(value))
    }
}

pub struct JiraEvent<T>(pub T);

impl<T, S> FromRequest<S> for JiraEvent<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: Request, _: &S) -> Result<Self, Self::Rejection> {
        let validate = config::get("WEBHOOK_VALIDATION") == "true";

        let (parts, body) = req.into_parts();

        let body_as_bytes = convert_body_to_bytes(body).await?;

        if validate {
            let token = config::get("JIRA_WEBHOOK_SECRET");
            let signature = parts.headers.get("x-hub-signature");

            validate_body(signature, &body_as_bytes, token)?;
        }

        let value = deseralise_body(body_as_bytes)?;

        Ok(JiraEvent(value))
    }
}

fn validate_body(
    signature_header: Option<&HeaderValue>,
    body: &Bytes,
    token: String,
) -> Result<(), (StatusCode, &'static str)> {
    let signature = signature_header
        .and_then(|v| v.to_str().ok())
        .ok_or(Response::BadRequest("Signature missing"))?
        .strip_prefix("sha256=")
        .ok_or(Response::BadRequest("Signature prefix missing"))?;

    let decoded_signature = hex::decode(signature).map_err(|err| {
        tracing::error!("Error decoding signature: ${err}");
        Response::BadRequest("Signature malformed")
    })?;

    let mac = HMAC::mac(body, token.as_bytes());

    if mac.ct_ne(&decoded_signature).into() {
        return Err(Response::BadRequest("Signature mismatch"));
    }

    Ok(())
}

async fn convert_body_to_bytes(
    body: axum::body::Body,
) -> Result<Bytes, (StatusCode, &'static str)> {
    axum::body::to_bytes(body, usize::MAX).await.map_err(|err| {
        tracing::error!("Error converting body to bytes: ${err}");
        Response::BadRequest("Error reading body")
    })
}

fn deseralise_body<T>(body: Bytes) -> Result<T, (StatusCode, &'static str)>
where
    T: DeserializeOwned,
{
    let deserializer = &mut serde_json::Deserializer::from_slice(&body);

    serde_path_to_error::deserialize(deserializer).map_err(|err| {
        tracing::error!("Error deserialising body: ${err}");
        Response::BadRequest("Error deserialising body")
    })
}

struct Response;

impl Response {
    #[allow(non_snake_case)]
    pub fn BadRequest(msg: &'static str) -> (StatusCode, &'static str) {
        (StatusCode::BAD_REQUEST, msg)
    }
}
