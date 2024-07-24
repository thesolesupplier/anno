use axum::{
    async_trait,
    body::Bytes,
    extract::{FromRequest, Request},
};
use hmac_sha256::HMAC;
use hyper::StatusCode;
use serde::de::DeserializeOwned;
use subtle::ConstantTimeEq;

use crate::utils::config;

pub struct GithubEvent<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for GithubEvent<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let validate = config::get("WEBHOOK_VALIDATION").is_ok_and(|v| v == "true");

        if !validate {
            let body = read_body_as_bytes(req, state).await?;
            let value = deseralise_body(body)?;

            return Ok(GithubEvent(value));
        }

        let token = config::get("GITHUB_WEBHOOK_SECRET").unwrap();

        let signature_sha256 = req
            .headers()
            .get("X-Hub-Signature-256")
            .and_then(|v| v.to_str().ok())
            .ok_or(Response::BadRequest("Signature missing"))?
            .strip_prefix("sha256=")
            .ok_or(Response::BadRequest("Signature prefix missing"))?;

        let signature = hex::decode(signature_sha256)
            .map_err(|_| Response::BadRequest("Signature malformed"))?;

        let body = read_body_as_bytes(req, state).await?;

        let mac = HMAC::mac(&body, token.as_bytes());

        if mac.ct_ne(&signature).into() {
            return Err(Response::BadRequest("Signature mismatch"));
        }

        let value = deseralise_body(body)?;

        Ok(GithubEvent(value))
    }
}

async fn read_body_as_bytes<S: Send + Sync>(
    req: Request,
    state: &S,
) -> Result<Bytes, (StatusCode, String)> {
    Bytes::from_request(req, state)
        .await
        .map_err(|_| Response::BadRequest("Error reading body"))
}

fn deseralise_body<T>(body: Bytes) -> Result<T, (StatusCode, String)>
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
    pub fn BadRequest(msg: &'static str) -> (StatusCode, String) {
        (StatusCode::BAD_REQUEST, msg.to_string())
    }
}
