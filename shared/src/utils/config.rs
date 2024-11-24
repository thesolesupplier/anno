use anyhow::Result;
use std::env;

pub fn load() {
    dotenv::dotenv().ok();
}

pub fn get(var_name: &str) -> Result<String> {
    env::var(var_name)
        // Also check for INPUT_ prefixed environment variables for GitHub action
        .or_else(|_| env::var(format!("INPUT_{var_name}")))
        .map_err(|_| panic!("{var_name} environment variable should be set"))
}
