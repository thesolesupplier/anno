use crate::utils::config;
use axum::{
    async_trait,
    body::Bytes,
    extract::{FromRequest, Request},
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

        if !validate {
            let body = read_body_as_bytes(req, state).await?;
            let value = deseralise_body(body)?;

            return Ok(GithubEvent(value));
        }

        let token = config::get("GITHUB_WEBHOOK_SECRET").unwrap();

        let body = validate_body("X-Hub-Signature-256", req, state, token)
            .await
            .map_err(Response::BadRequest)?;

        Ok(GithubEvent(body))
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

        if !validate {
            let body = read_body_as_bytes(req, state).await?;
            let value = deseralise_body(body)?;

            return Ok(JiraEvent(value));
        }

        let token = config::get("JIRA_WEBHOOK_SECRET").unwrap();

        let body = validate_body("x-hub-signature", req, state, token)
            .await
            .map_err(Response::BadRequest)?;

        Ok(JiraEvent(body))
    }
}

async fn validate_body<S: Send + Sync, T: DeserializeOwned>(
    header_name: &'static str,
    req: Request,
    state: &S,
    token: String,
) -> Result<T, &'static str> {
    let signature_sha256 = req
        .headers()
        .get(header_name)
        .and_then(|v| v.to_str().ok())
        .ok_or("Signature missing")?
        .strip_prefix("sha256=")
        .ok_or("Signature prefix missing")?;

    let signature = hex::decode(signature_sha256).map_err(|_| "Signature malformed")?;

    let body = read_body_as_bytes(req, state)
        .await
        .map_err(|_| "Unable to read body")?;

    let mac = HMAC::mac(&body, token.as_bytes());

    if mac.ct_ne(&signature).into() {
        return Err("Signature mismatch");
    }

    let value = deseralise_body(body).map_err(|_| "Error deserialising body")?;

    Ok(value)
}

async fn read_body_as_bytes<S: Send + Sync>(
    req: Request,
    state: &S,
) -> Result<Bytes, (StatusCode, &'static str)> {
    Bytes::from_request(req, state)
        .await
        .map_err(|_| Response::BadRequest("Error reading body"))
}

fn deseralise_body<T>(body: Bytes) -> Result<T, (StatusCode, &'static str)>
where
    T: DeserializeOwned,
{
    let deserializer = &mut serde_json::Deserializer::from_slice(&body);
    let value = serde_path_to_error::deserialize(deserializer)
        .map_err(|_| Response::BadRequest("Error deserialising body"))?;

    Ok(value)
}

struct Response;

impl Response {
    #[allow(non_snake_case)]
    pub fn BadRequest(msg: &'static str) -> (StatusCode, &'static str) {
        (StatusCode::BAD_REQUEST, msg)
    }
}
