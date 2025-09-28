use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_ENGINE};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_encode};
use sha1::Sha1;
use thiserror::Error;

use crate::config::AppConfig;

const PATH_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~')
    .remove(b'/');

#[derive(Debug, Error)]
pub enum SigningError {
    #[error("Bucket name is required when default bucket is not configured")]
    MissingBucket,
    #[error("HMAC signing error")]
    SigningFailure,
}

type HmacSha1 = Hmac<Sha1>;

pub struct SignedUrl {
    pub url: String,
    #[allow(dead_code)]
    pub expires_at: DateTime<Utc>,
}

pub fn build_signed_url(
    config: &AppConfig,
    bucket_override: Option<&str>,
    object_key: &str,
    expires_at: DateTime<Utc>,
    download_filename: Option<&str>,
    endpoint_override: Option<&str>,
) -> Result<SignedUrl, SigningError> {
    let bucket = bucket_override
        .map(|value| value.to_string())
        .or_else(|| config.aliyun_default_bucket.clone())
        .ok_or(SigningError::MissingBucket)?;

    let encoded_key = percent_encode_path(object_key);

    let expires = expires_at.timestamp();

    let mut canonical_resource = format!("/{}/{}", bucket, encoded_key);
    let mut additional_query = String::new();
    if let Some(filename) = download_filename {
        if !filename.trim().is_empty() {
            let sanitized = filename.replace('"', "");
            let disposition = format!("attachment; filename=\"{}\"", sanitized);
            let encoded_disposition =
                percent_encode(disposition.as_bytes(), NON_ALPHANUMERIC).to_string();
            canonical_resource = format!(
                "{}?response-content-disposition={}",
                canonical_resource, encoded_disposition
            );
            additional_query = format!("&response-content-disposition={}", encoded_disposition);
        }
    }

    let string_to_sign = format!("GET\n\n\n{}\n{}", expires, canonical_resource);

    let mut mac = HmacSha1::new_from_slice(config.aliyun_access_key_secret.as_bytes())
        .map_err(|_| SigningError::SigningFailure)?;
    mac.update(string_to_sign.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_b64 = BASE64_ENGINE.encode(signature);

    let endpoint = endpoint_override
        .map(|e| e.to_string())
        .unwrap_or_else(|| config.aliyun_default_endpoint.clone());
    let host = build_oss_host(&bucket, &endpoint);
    let access_key_encoded =
        percent_encode(config.aliyun_access_key_id.as_bytes(), NON_ALPHANUMERIC).to_string();
    let signature_encoded = percent_encode(signature_b64.as_bytes(), NON_ALPHANUMERIC).to_string();

    let url = format!(
        "{host}/{object}?OSSAccessKeyId={access_key}&Expires={expires}&Signature={signature}{extra}",
        host = host,
        object = encoded_key,
        access_key = access_key_encoded,
        expires = expires,
        signature = signature_encoded,
        extra = additional_query,
    );

    Ok(SignedUrl { url, expires_at })
}

fn build_oss_host(bucket: &str, endpoint: &str) -> String {
    let trimmed = endpoint.trim().trim_end_matches('/');
    if trimmed.contains("{bucket}") {
        trimmed.replace("{bucket}", bucket)
    } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        format!("{}/{}", trimmed, bucket)
    } else {
        format!("https://{}.{}", bucket, trimmed)
    }
}

fn percent_encode_path(value: &str) -> String {
    percent_encode(value.as_bytes(), PATH_ENCODE_SET).to_string()
}
