use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::{AuthUser, generate_token};
use crate::oauth::{OAuthError, check_admin_permission, exchange_code_for_token, fetch_user_info};
use crate::oss_client::OssClient;
use crate::oss_client::{SigningError, build_signed_url};
use crate::state::{AppState, DownloadTicket};

pub fn create_router(state: AppState) -> Router {
    let download_prefix = format!("/{}", state.config.download_prefix);

    Router::new()
        .route("/healthz", get(health_check))
        // OAuth2 authentication routes
        .route("/api/oauth/callback", get(oauth_callback))
        // Frontend domain routes - gurl.honahec.cc (management functions)
        .route("/sign", post(create_signed_link))
        .route("/buckets", get(list_buckets))
        .route("/objects", get(list_objects))
        .route("/links", get(list_links))
        .route("/links/:id", get(get_link_info))
        .route("/links/:id", axum::routing::delete(delete_link))
        .route("/cleanup", post(cleanup_expired_links))
        // Backend domain routes - api.honahec.cc (public access)
        .nest(
            &download_prefix,
            Router::new().route("/:id", get(resolve_download)),
        )
        .with_state(state)
}

async fn health_check() -> &'static str {
    "ok"
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
    #[allow(dead_code)]
    pub state: String,
    pub code_verifier: String,
}

#[derive(Debug, Serialize)]
pub struct OAuthCallbackResponse {
    pub token: String,
    pub expires_in: i64,
    pub username: String,
}

// OAuth2 callback handler
async fn oauth_callback(
    State(state): State<AppState>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Json<OAuthCallbackResponse>, ApiError> {
    // Exchange authorization code for access token
    let token_response = exchange_code_for_token(&state.config, &query.code, &query.code_verifier)
        .await
        .map_err(ApiError::OAuth)?;

    // Fetch user information
    let user_info = fetch_user_info(&state.config, &token_response.access_token)
        .await
        .map_err(ApiError::OAuth)?;

    // Check admin permission
    check_admin_permission(&user_info).map_err(ApiError::OAuth)?;

    // Generate JWT token
    let jwt_token = generate_token(&user_info.username, &state.config)
        .map_err(|_| ApiError::Internal("Failed to generate token".to_string()))?;

    Ok(Json(OAuthCallbackResponse {
        token: jwt_token,
        expires_in: state.config.jwt_exp_minutes * 60,
        username: user_info.username,
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateLinkRequest {
    pub object_key: String,
    pub bucket: Option<String>,
    pub expires_in_seconds: i64,
    pub max_downloads: Option<u32>,
    pub download_filename: Option<String>,
    pub endpoint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateLinkResponse {
    pub id: Uuid,
    pub url: String,
    pub expires_at: String,
    pub max_downloads: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct ListLinksQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ListLinksResponse {
    pub links: Vec<DownloadLinkResponse>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct DownloadLinkResponse {
    pub id: String,
    pub object_key: String,
    pub bucket: Option<String>,
    pub expires_at: String,
    pub max_downloads: Option<i64>,
    pub downloads_served: i64,
    pub created_at: String,
    pub download_filename: Option<String>,
    pub endpoint: Option<String>,
    pub is_expired: bool,
    pub download_url: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct CleanupResponse {
    pub deleted_count: u64,
}

async fn create_signed_link(
    _user: AuthUser,
    State(state): State<AppState>,
    Json(payload): Json<CreateLinkRequest>,
) -> Result<Json<CreateLinkResponse>, ApiError> {
    if payload.object_key.is_empty() {
        return Err(ApiError::BadRequest(
            "Object key cannot be empty".to_string(),
        ));
    }

    let expires_in = if payload.expires_in_seconds > 0 {
        payload.expires_in_seconds
    } else {
        state.config.default_expiry_secs
    };

    let expires_at = Utc::now() + Duration::seconds(expires_in);

    let id = Uuid::new_v4();
    let ticket = DownloadTicket {
        id,
        bucket_override: payload.bucket.clone(),
        object_key: payload.object_key.clone(),
        expires_at,
        max_downloads: payload.max_downloads,
        downloads_served: 0,
        created_at: Utc::now(),
        download_filename: payload.download_filename.clone(),
        endpoint_override: payload.endpoint.clone(),
    };

    // Store to database
    state
        .database
        .create_download_link(
            id,
            payload.object_key,
            payload.bucket,
            expires_at,
            payload.max_downloads,
            payload.download_filename,
            payload.endpoint,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    // Store ticket to memory
    {
        let mut tickets = state.tickets.write().await;
        tickets.insert(id, ticket);
    }

    let download_url = format!(
        "{}/{}",
        state.config.public_base_url.trim_end_matches('/'),
        format!("{}/{}", state.config.download_prefix, id)
    );

    Ok(Json(CreateLinkResponse {
        id,
        url: download_url,
        expires_at: expires_at.to_rfc3339(),
        max_downloads: payload.max_downloads,
    }))
}

async fn resolve_download(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<Redirect, (StatusCode, String)> {
    let now = Utc::now();

    // Get ticket
    let tickets = state.tickets.read().await;
    let ticket = tickets
        .get(&id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Download link not found".to_string()))?;

    // Check if expired
    if now > ticket.expires_at {
        return Err((StatusCode::GONE, "Download link has expired".to_string()));
    }

    // Check download count limit
    if let Some(max_downloads) = ticket.max_downloads {
        if ticket.downloads_served >= max_downloads {
            return Err((
                StatusCode::TOO_MANY_REQUESTS,
                "Download limit exceeded".to_string(),
            ));
        }
    }

    drop(tickets);

    // Update download count
    let mut tickets_mut = state.tickets.write().await;
    if let Some(ticket_mut) = tickets_mut.get_mut(&id) {
        ticket_mut.downloads_served += 1;
    }
    drop(tickets_mut);

    // Update download count in database
    let _ = state.database.increment_downloads(&id.to_string()).await;

    // Re-fetch ticket info for generating signed URL
    let tickets = state.tickets.read().await;
    let ticket = tickets.get(&id).unwrap();

    // Generate signed download URL
    let signed_url = build_signed_url(
        &state.config,
        ticket.bucket_override.as_deref(),
        &ticket.object_key,
        ticket.expires_at,
        ticket.download_filename.as_deref(),
        ticket.endpoint_override.as_deref(),
    )
    .map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to generate download URL".to_string(),
        )
    })?;

    Ok(Redirect::temporary(&signed_url.url))
}

// Get links list
async fn list_links(
    _user: AuthUser,
    Query(params): Query<ListLinksQuery>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let links = state
        .database
        .list_download_links(params.limit, params.offset)
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    let download_links: Vec<DownloadLinkResponse> = links
        .into_iter()
        .map(|link| DownloadLinkResponse {
            id: link.id.clone(),
            object_key: link.object_key,
            bucket: link.bucket,
            expires_at: link.expires_at.to_rfc3339(),
            max_downloads: link.max_downloads,
            downloads_served: link.downloads_served,
            created_at: link.created_at.to_rfc3339(),
            download_filename: link.download_filename,
            endpoint: link.endpoint,
            is_expired: link.is_expired,
            download_url: format!(
                "{}/{}",
                state.config.public_base_url.trim_end_matches('/'),
                format!("{}/{}", state.config.download_prefix, link.id)
            ),
        })
        .collect();

    let response = Json(ListLinksResponse {
        total: download_links.len(),
        links: download_links,
    });

    let mut response = response.into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store, no-cache, must-revalidate"),
    );
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response
        .headers_mut()
        .insert(header::EXPIRES, HeaderValue::from_static("0"));

    Ok(response)
}

// Get single link info
async fn get_link_info(
    _user: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<DownloadLinkResponse>, ApiError> {
    let link = state
        .database
        .get_download_link(&id)
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::BadRequest("Link not found".to_string()))?;

    let response = DownloadLinkResponse {
        id: link.id.clone(),
        object_key: link.object_key,
        bucket: link.bucket,
        expires_at: link.expires_at.to_rfc3339(),
        max_downloads: link.max_downloads,
        downloads_served: link.downloads_served,
        created_at: link.created_at.to_rfc3339(),
        download_filename: link.download_filename,
        endpoint: link.endpoint,
        is_expired: link.is_expired,
        download_url: format!(
            "{}/{}",
            state.config.public_base_url.trim_end_matches('/'),
            format!("{}/{}", state.config.download_prefix, link.id)
        ),
    };

    Ok(Json(response))
}

// Delete link
async fn delete_link(
    _user: AuthUser,
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<DeleteResponse>, ApiError> {
    let deleted = state
        .database
        .delete_download_link(&id)
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    if deleted {
        // Also remove from memory
        if let Ok(uuid) = Uuid::parse_str(&id) {
            let mut tickets = state.tickets.write().await;
            tickets.remove(&uuid);
        }

        Ok(Json(DeleteResponse {
            success: true,
            message: "Link deleted successfully".to_string(),
        }))
    } else {
        Ok(Json(DeleteResponse {
            success: false,
            message: "Link not found".to_string(),
        }))
    }
}

// Clean up expired links
async fn cleanup_expired_links(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<CleanupResponse>, ApiError> {
    let deleted_count = state
        .database
        .delete_expired_links()
        .await
        .map_err(|e| ApiError::Internal(format!("Database error: {}", e)))?;

    // Also clean up expired tickets in memory
    let now = Utc::now();
    let mut tickets = state.tickets.write().await;
    tickets.retain(|_, ticket| {
        let not_time_expired = now <= ticket.expires_at;
        let not_download_exceeded = ticket
            .max_downloads
            .map_or(true, |max| ticket.downloads_served < max);
        not_time_expired && not_download_exceeded
    });

    Ok(Json(CleanupResponse { deleted_count }))
}

async fn list_buckets(
    _user: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<crate::oss_client::ListBucketsResponse>, ApiError> {
    let client = OssClient::new(state.config.as_ref())
        .map_err(|e| ApiError::Internal(format!("Failed to create OSS client: {}", e)))?;

    let response = client
        .list_buckets()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list buckets: {}", e)))?;

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct ListObjectsQuery {
    pub bucket: String,
    pub prefix: Option<String>,
    #[serde(rename = "continuation-token")]
    pub continuation_token: Option<String>,
}

async fn list_objects(
    _user: AuthUser,
    State(state): State<AppState>,
    Query(query): Query<ListObjectsQuery>,
) -> Result<Json<crate::oss_client::ListObjectsResponse>, ApiError> {
    if query.bucket.is_empty() {
        return Err(ApiError::BadRequest("Bucket name is required".to_string()));
    }

    let client = OssClient::new(state.config.as_ref())
        .map_err(|e| ApiError::Internal(format!("Failed to create OSS client: {}", e)))?;

    let response = client
        .list_objects(
            &query.bucket,
            query.prefix.as_deref(),
            query.continuation_token.as_deref(),
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to list objects: {}", e)))?;

    Ok(Json(response))
}

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    Internal(String),
    #[allow(dead_code)]
    Signing(SigningError),
    #[allow(dead_code)]
    Unauthorized,
    OAuth(OAuthError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            ApiError::Signing(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            ApiError::OAuth(err) => (StatusCode::UNAUTHORIZED, err.to_string()),
        };

        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        (status, Json(ErrorResponse { message })).into_response()
    }
}
