use std::env;
use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_host: String,
    pub api_port: u16,
    pub public_base_url: String,
    pub download_prefix: String,
    pub aliyun_access_key_id: String,
    pub aliyun_access_key_secret: String,
    pub aliyun_default_endpoint: Option<String>,
    pub aliyun_default_bucket: Option<String>,
    pub default_expiry_secs: i64,
    pub jwt_secret: String,
    pub jwt_exp_minutes: i64,
    pub oauth_client_id: String,
    pub oauth_client_secret: String,
    #[allow(dead_code)]
    pub oauth_authorize_url: String,
    pub oauth_token_url: String,
    pub oauth_userinfo_url: String,
    #[allow(dead_code)]
    pub oauth_redirect_uri: String,
    pub cors_allowed_origins: Vec<String>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing environment variable {0}")]
    MissingVar(&'static str),
    #[error("Invalid value for {0}: {1}")]
    ParseError(&'static str, String),
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let api_host = env::var("API_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let api_port = parse_with_default("API_PORT", 8080u16)?;
        let public_base_url =
            env::var("PUBLIC_BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
        let public_base_url = normalize_base_url(public_base_url);
        let download_prefix =
            env::var("DOWNLOAD_PATH_PREFIX").unwrap_or_else(|_| "download".to_string());
        let download_prefix = trim_slashes(&download_prefix).to_string();

        let aliyun_access_key_id = require_env("ALIYUN_ACCESS_KEY_ID")?;
        let aliyun_access_key_secret = require_env("ALIYUN_ACCESS_KEY_SECRET")?;
        let aliyun_default_endpoint = env::var("ALIYUN_DEFAULT_ENDPOINT")
            .ok()
            .filter(|s| !s.is_empty());
        let aliyun_default_bucket = env::var("ALIYUN_DEFAULT_BUCKET")
            .ok()
            .filter(|s| !s.is_empty());

        let default_expiry_secs = parse_with_default("DEFAULT_EXPIRY_SECS", 3600i64)?;
        let jwt_secret = require_env("JWT_SECRET")?;
        let jwt_exp_minutes = parse_with_default("JWT_EXP_MINUTES", 60i64)?;

        let oauth_client_id = require_env("OAUTH_CLIENT_ID")?;
        let oauth_client_secret = require_env("OAUTH_CLIENT_SECRET")?;
        let oauth_authorize_url = env::var("OAUTH_AUTHORIZE_URL")
            .unwrap_or_else(|_| "https://sso.honahec.cc/oauth/authorize/".to_string());
        let oauth_token_url = env::var("OAUTH_TOKEN_URL")
            .unwrap_or_else(|_| "https://sso.honahec.cc/oauth/token/".to_string());
        let oauth_userinfo_url = env::var("OAUTH_USERINFO_URL")
            .unwrap_or_else(|_| "https://sso.honahec.cc/oauth/userinfo/".to_string());
        let oauth_redirect_uri = require_env("OAUTH_REDIRECT_URI")?;

        let cors_allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
            .map(|value| parse_origins(&value))
            .unwrap_or_else(|_| vec!["*".to_string()]);

        Ok(Self {
            api_host,
            api_port,
            public_base_url,
            download_prefix,
            aliyun_access_key_id,
            aliyun_access_key_secret,
            aliyun_default_endpoint,
            aliyun_default_bucket,
            default_expiry_secs,
            jwt_secret,
            jwt_exp_minutes,
            oauth_client_id,
            oauth_client_secret,
            oauth_authorize_url,
            oauth_token_url,
            oauth_userinfo_url,
            oauth_redirect_uri,
            cors_allowed_origins,
        })
    }

    #[allow(dead_code)]
    pub fn download_base_url(&self) -> String {
        format!("{}/{}/", self.public_base_url, self.download_prefix)
    }
}

fn require_env(key: &'static str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| ConfigError::MissingVar(key))
}

fn parse_with_default<T>(key: &'static str, default_value: T) -> Result<T, ConfigError>
where
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    match env::var(key) {
        Ok(raw) => raw
            .parse::<T>()
            .map_err(|err| ConfigError::ParseError(key, err.to_string())),
        Err(_) => Ok(default_value),
    }
}

fn normalize_base_url(url: String) -> String {
    let trimmed = url.trim_end_matches('/');
    if trimmed.is_empty() {
        "http://localhost:8080".to_string()
    } else {
        trimmed.to_string()
    }
}

fn trim_slashes(value: &str) -> &str {
    value.trim_matches('/')
}

fn parse_origins(value: &str) -> Vec<String> {
    if value.trim() == "*" {
        vec!["*".to_string()]
    } else {
        value
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}
