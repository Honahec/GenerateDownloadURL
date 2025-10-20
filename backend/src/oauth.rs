use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserInfo {
    pub sub: String,
    pub username: String,
    pub email: Option<String>,
    pub permissions: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum OAuthError {
    #[allow(dead_code)]
    InvalidState,
    #[allow(dead_code)]
    InvalidSession,
    TokenExchangeFailed(String),
    UserInfoFailed(String),
    PermissionDenied,
    #[allow(dead_code)]
    InvalidResponse(String),
}

impl std::fmt::Display for OAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OAuthError::InvalidState => write!(f, "Invalid OAuth state parameter"),
            OAuthError::InvalidSession => write!(f, "OAuth session not found or expired"),
            OAuthError::TokenExchangeFailed(msg) => write!(f, "Token exchange failed: {}", msg),
            OAuthError::UserInfoFailed(msg) => write!(f, "Failed to fetch user info: {}", msg),
            OAuthError::PermissionDenied => write!(f, "User does not have admin permission"),
            OAuthError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
        }
    }
}

impl std::error::Error for OAuthError {}

pub async fn exchange_code_for_token(
    config: &AppConfig,
    code: &str,
    code_verifier: &str,
) -> Result<TokenResponse, OAuthError> {
    let client = reqwest::Client::new();

    let mut params = HashMap::new();
    params.insert("grant_type", "authorization_code");
    params.insert("code", code);
    params.insert("redirect_uri", config.oauth_redirect_uri.as_str());
    params.insert("client_id", config.oauth_client_id.as_str());
    params.insert("client_secret", config.oauth_client_secret.as_str());
    params.insert("code_verifier", code_verifier);

    let response = client
        .post(&config.oauth_token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| OAuthError::TokenExchangeFailed(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(OAuthError::TokenExchangeFailed(format!(
            "HTTP {}: {}",
            status, text
        )));
    }

    response
        .json::<TokenResponse>()
        .await
        .map_err(|e| OAuthError::TokenExchangeFailed(e.to_string()))
}

pub async fn fetch_user_info(
    config: &AppConfig,
    access_token: &str,
) -> Result<UserInfo, OAuthError> {
    let client = reqwest::Client::new();

    let response = client
        .get(&config.oauth_userinfo_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| OAuthError::UserInfoFailed(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        return Err(OAuthError::UserInfoFailed(format!(
            "HTTP {}: {}",
            status, text
        )));
    }

    response
        .json::<UserInfo>()
        .await
        .map_err(|e| OAuthError::UserInfoFailed(e.to_string()))
}

pub fn check_admin_permission(user_info: &UserInfo) -> Result<(), OAuthError> {
    // Check if user has admin_user permission set to true
    if let Some(permissions) = &user_info.permissions {
        if let Some(admin_user) = permissions.get("admin_user") {
            if admin_user.as_bool() == Some(true) {
                return Ok(());
            }
        }
    }

    Err(OAuthError::PermissionDenied)
}
