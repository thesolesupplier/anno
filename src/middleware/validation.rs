use crate::utils::config;
use axum::{
    async_trait,
    body::Bytes,
    extract::{FromRequest, Request},
    http::HeaderValue,
};
use hmac_sha256::HMAC;
use hyper::StatusCode;
use serde::de::DeserializeOwned;
use subtle::ConstantTimeEq;

pub struct GithubEvent<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for GithubEvent<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let validate = config::get("WEBHOOK_VALIDATION").is_ok_and(|v| v == "true");

        let signature = req.headers().get("X-Hub-Signature-256").cloned();
        let body = convert_body_to_bytes(req, state).await?;

        if validate {
            let token = config::get("GITHUB_WEBHOOK_SECRET").unwrap();
            validate_body(signature, &body, token).await?;
        }

        let value = deseralise_body(body)?;

        Ok(GithubEvent(value))
    }
}

pub struct JiraEvent<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for JiraEvent<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let validate = config::get("WEBHOOK_VALIDATION").is_ok_and(|v| v == "true");

        let signature = req.headers().get("x-hub-signature").cloned();
        let body = convert_body_to_bytes(req, state).await?;

        if validate {
            let token = config::get("JIRA_WEBHOOK_SECRET").unwrap();
            validate_body(signature, &body, token).await?;
        }

        let value = deseralise_body(body)?;

        Ok(JiraEvent(value))
    }
}

async fn validate_body(
    signature_header: Option<HeaderValue>,
    body: &Bytes,
    token: String,
) -> Result<(), (StatusCode, &'static str)> {
    let signature = signature_header.ok_or(Response::BadRequest("Signature missing"))?;

    let signature = signature
        .to_str()
        .map_err(|_| Response::BadRequest("Signature malformed"))?
        .strip_prefix("sha256=")
        .ok_or(Response::BadRequest("Signature prefix missing"))?;

    let signature = hex::decode(signature).map_err(|err| {
        tracing::error!("Error decoding signature: ${err}");
        Response::BadRequest("Signature malformed")
    })?;

    let mac = HMAC::mac(body, token.as_bytes());

    if mac.ct_ne(&signature).into() {
        return Err(Response::BadRequest("Signature mismatch"));
    }

    Ok(())
}

async fn convert_body_to_bytes<S: Send + Sync>(
    req: Request,
    state: &S,
) -> Result<Bytes, (StatusCode, &'static str)> {
    Bytes::from_request(req, state).await.map_err(|err| {
        tracing::error!("Error converting body to bytes: ${err}");
        Response::BadRequest("Error reading body")
    })
}

fn deseralise_body<T>(body: Bytes) -> Result<T, (StatusCode, &'static str)>
where
    T: DeserializeOwned,
{
    let deserializer = &mut serde_json::Deserializer::from_slice(&body);
    let value = serde_path_to_error::deserialize(deserializer).map_err(|err| {
        tracing::error!("Error deserialising body: ${err}");
        Response::BadRequest("Error deserialising body")
    })?;

    Ok(value)
}

struct Response;

impl Response {
    #[allow(non_snake_case)]
    pub fn BadRequest(msg: &'static str) -> (StatusCode, &'static str) {
        (StatusCode::BAD_REQUEST, msg)
    }
}
