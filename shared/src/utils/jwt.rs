use crate::utils::config;
use base64::prelude::*;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm::RS256, EncodingKey, Header};
use serde::{Deserialize, Serialize};

pub fn create_github_token() -> String {
    let gh_app_id = config::get("GITHUB_APP_ID");
    let gh_app_private_key_base64 = config::get("GITHUB_APP_PRIVATE_KEY_BASE64");

    let private_key = BASE64_STANDARD
        .decode(gh_app_private_key_base64)
        .expect("Private key to be base64");

    let now = Utc::now();
    let claims = Claims {
        iss: gh_app_id,
        iat: (now - Duration::seconds(60)).timestamp(),
        exp: (now + Duration::minutes(10)).timestamp(),
    };

    let key = EncodingKey::from_rsa_pem(&private_key).expect("Private key to be valid");

    jsonwebtoken::encode(&Header::new(RS256), &claims, &key).expect("Encoding to be successful")
}

#[derive(Serialize, Deserialize)]
struct Claims {
    iss: String,
    iat: i64,
    exp: i64,
}
