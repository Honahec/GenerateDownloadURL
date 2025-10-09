use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_ENGINE};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use percent_encoding::{AsciiSet, CONTROLS, NON_ALPHANUMERIC, percent_encode};
use reqwest;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::Sha256;
use sha256::digest;
use std::collections::BTreeMap;
use thiserror::Error;

type HmacSha1 = Hmac<Sha1>;
type HmacSha256 = Hmac<Sha256>;

use crate::config::AppConfig;

const UNRESERVED: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'!')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'=')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

// 用于查询参数编码
const QUERY: &AsciiSet = UNRESERVED;

const PATH_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~')
    .remove(b'/');

#[derive(Debug, Error)]
pub enum OssError {
    #[error("HTTP request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("XML parsing failed: {0}")]
    XmlParsingFailed(String),
    #[error("Missing endpoint configuration")]
    MissingEndpoint,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Bucket {
    pub name: String,
    pub location: String,
    pub creation_date: String,
    pub storage_class: String, 
    pub extranet_endpoint: String,
    pub intranet_endpoint: String,
}

#[derive(Debug, Serialize)]
pub struct ListBucketsResponse {
    pub buckets: Vec<Bucket>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub key: String,
    pub last_modified: String,
    pub size: u64,
    pub storage_class: String,
}

#[derive(Debug, Serialize)]
pub struct ListObjectsResponse {
    pub objects: Vec<ObjectInfo>,
    pub is_truncated: bool,
    pub next_continuation_token: Option<String>,
}

#[derive(Debug, Error)]
pub enum SigningError {
    #[error("Bucket name is required when default bucket is not configured")]
    MissingBucket,
    #[error("HMAC signing error")]
    SigningFailure,
    #[error("Endpoint is required when default endpoint is not configured")]
    MissingEndpoint,
}

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

    let canonical_resource = format!("/{}/{}", bucket, encoded_key);
    let mut additional_query = String::new();
    let canonical_oss_headers = String::new();

    if let Some(filename) = download_filename {
        if !filename.trim().is_empty() {
            let sanitized = filename.replace('"', "");
            let disposition = format!("attachment; filename=\"{}\"", sanitized);
            let encoded_disposition =
                percent_encode(disposition.as_bytes(), NON_ALPHANUMERIC).to_string();
            additional_query = format!("&response-content-disposition={}", encoded_disposition);
        }
    }

    let string_to_sign = format!("GET


{}
{}{}", expires, canonical_oss_headers, canonical_resource);

    let mut mac = HmacSha1::new_from_slice(config.aliyun_access_key_secret.as_bytes())
        .map_err(|_| SigningError::SigningFailure)?;
    mac.update(string_to_sign.as_bytes());
    let signature = mac.finalize().into_bytes();
    let signature_b64 = BASE64_ENGINE.encode(signature);

    let endpoint = endpoint_override
        .map(|e| e.to_string())
        .or_else(|| config.aliyun_default_endpoint.clone())
        .ok_or(SigningError::MissingEndpoint)?;

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

pub struct OssClient {
    access_key_id: String,
    access_key_secret: String,
    endpoint: String,
    client: reqwest::Client,
}

impl OssClient {
    pub fn new(config: &AppConfig) -> Result<Self, OssError> {
        let endpoint = config
            .aliyun_default_endpoint
            .clone()
            .ok_or(OssError::MissingEndpoint)?;

        Ok(Self {
            access_key_id: config.aliyun_access_key_id.clone(),
            access_key_secret: config.aliyun_access_key_secret.clone(),
            endpoint,
            client: reqwest::Client::new(),
        })
    }

    pub async fn list_buckets(&self) -> Result<ListBucketsResponse, OssError> {
        let now = Utc::now();
        let date_header = now.format("%a, %d %b %Y %H:%M:%S GMT").to_string();
        let host = self.get_host();

        // 使用OSS V1签名
        let authorization = self.build_v1_authorization("GET", "", "", &date_header, "", "/")?;
        let url = format!("https://{}", host);

        let response = self
            .client
            .get(&url)
            .header("Date", &date_header)
            .header("Host", &host)
            .header("Authorization", &authorization)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(OssError::XmlParsingFailed(format!(
                "OSS API returned status {}: {}",
                status, text
            )));
        }

        self.parse_buckets_xml(&text)
    }

    pub async fn list_objects(
        &self,
        bucket_name: &str,
        prefix: Option<&str>,
        continuation_token: Option<&str>,
    ) -> Result<ListObjectsResponse, OssError> {
        let buckets_response = self.list_buckets().await?;
        let bucket = buckets_response
            .buckets
            .iter()
            .find(|b| b.name == bucket_name)
            .ok_or_else(|| {
                OssError::XmlParsingFailed(format!("Bucket '{}' not found", bucket_name))
            })?;

        let now = Utc::now();
        let date_header = now.format("%a, %d %b %Y %H:%M:%S GMT").to_string();

        // 使用三级域名格式：bucket-name.oss-region.aliyuncs.com
        let endpoint_host = self.extract_host_from_endpoint(&bucket.extranet_endpoint);
        let host = format!("{}.{}", bucket_name, endpoint_host); // 三级域名

        let mut query_params = BTreeMap::new();
        // 使用ListObjectsV2 API
        query_params.insert("list-type".to_string(), "2".to_string());
        if let Some(p) = prefix {
            query_params.insert("prefix".to_string(), p.to_string());
        }
        if let Some(token) = continuation_token {
            query_params.insert("continuation-token".to_string(), token.to_string());
        }
        query_params.insert("max-keys".to_string(), "1000".to_string());

        // 构建HTTP请求的查询字符串（需要URL编码）
        let query_string = if query_params.is_empty() {
            String::new()
        } else {
            let encoded_params: Vec<String> = query_params
                .iter()
                .map(|(k, v)| format!("{}={}", 
                    percent_encode(k.as_bytes(), QUERY),
                    percent_encode(v.as_bytes(), QUERY)))
                .collect();
            format!("?{}", encoded_params.join("&"))
        };

        // 构建 CanonicalizedResource
        // 根据OSS规范，对于虚拟主机风格的请求�?
        // - 如果�?bucket.oss-region.aliyuncs.com，CanonicalizedResource = "/"
        // - 只有OSS子资源才需要包含在签名�?
        let canonical_resource = format!("/{}/", bucket_name);
        
        // OSS子资源列表（只有这些查询参数才需要包含在签名中）
        let oss_sub_resources = [
            "acl", "lifecycle", "location", "logging", "notification", "partNumber",
            "policy", "requestPayment", "torrent", "uploadId", "uploads", "versionId",
            "versioning", "versions", "website", "cors", "delete", "restore", "tagging",
            "encryption", "inventory", "select", "x-oss-process", "continuation-token",
        ];
        
        // 检查是否有OSS子资源需要包含在签名�?
        let mut sub_resource_params = BTreeMap::new();
        for (key, value) in &query_params {
            if oss_sub_resources.contains(&key.as_str()) {
                sub_resource_params.insert(key.clone(), value.clone());
            }
        }
        
        // 如果有OSS子资源，添加到canonical resource
        let final_canonical_resource = if !sub_resource_params.is_empty() {
            let mut resource = canonical_resource;
            resource.push('?');
            let sorted_params: Vec<String> = sub_resource_params
                .iter()
                .map(|(k, v)| {
                    let encoded_key = percent_encode(k.as_bytes(), QUERY).to_string();
                    if v.is_empty() {
                        encoded_key
                    } else {
                        let encoded_value = percent_encode(v.as_bytes(), QUERY).to_string();
                        format!("{}={}", encoded_key, encoded_value)
                    }
                })
                .collect();
            resource.push_str(&sorted_params.join("&"));
            resource
        } else {
            canonical_resource
        };
        
        let authorization = self.build_v1_authorization(
            "GET", "", "", &date_header, "", &final_canonical_resource
        )?;
        
        let url = format!("https://{}{}", host, query_string);

        let response = self
            .client
            .get(&url)
            .header("Date", &date_header)
            .header("Host", &host)
            .header("Authorization", &authorization)
            .send()
            .await?;

        let status = response.status();
        let text = response.text().await?;

        if !status.is_success() {
            return Err(OssError::XmlParsingFailed(format!(
                "OSS API returned status {}: {}",
                status, text
            )));
        }

        self.parse_objects_xml(&text)
    }

    fn get_host(&self) -> String {
        let trimmed = self
            .endpoint
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/');
        trimmed.to_string()
    }

    fn extract_host_from_endpoint(&self, endpoint: &str) -> String {
        endpoint
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string()
    }

    /// 构建OSS V1签名的Authorization header
    fn build_v1_authorization(
        &self,
        method: &str,
        content_md5: &str,
        content_type: &str,
        date: &str,
        canonical_oss_headers: &str,
        canonical_resource: &str,
    ) -> Result<String, OssError> {
        // OSS V1签名规范
        // StringToSign = VERB + "\n" + CONTENT-MD5 + "\n" + CONTENT-TYPE + "\n" + DATE + "\n" + CanonicalizedOSSHeaders + CanonicalizedResource
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}\n{}{}",
            method, content_md5, content_type, date, canonical_oss_headers, canonical_resource
        );

        // 使用HMAC-SHA1计算签名
        let mut mac = HmacSha1::new_from_slice(self.access_key_secret.as_bytes())
            .map_err(|_| OssError::XmlParsingFailed("HMAC signing error".to_string()))?;
        mac.update(string_to_sign.as_bytes());
        let signature = mac.finalize().into_bytes();
        let signature_b64 = BASE64_ENGINE.encode(signature);

        let authorization = format!("OSS {}:{}", self.access_key_id, signature_b64);

        Ok(authorization)
    }



    #[allow(dead_code)]
    fn  build_v4_authorization_advanced(
        &self,
        method: &str,
        iso_datetime: &str,
        host: &str,
        path: &str,
        query_string: &str,
        additional_headers: &BTreeMap<String, String>,
    ) -> Result<String, OssError> {
        let date_only = &iso_datetime[0..8];

        // 提取region
        let region = self.extract_region_from_host(host);

        // 1. 构建canonical_query_string - 对查询参数排序和URL编码
        let canonical_query_string = self.build_canonical_query_string(query_string);

        // 2. 构建canonical_headers - 规范化headers
        let mut headers = BTreeMap::new();
        
        // 必选的headers - 所有x-oss-*头必须参与签名
        headers.insert("host".to_string(), host.to_string());
        headers.insert(
            "x-oss-content-sha256".to_string(),
            "UNSIGNED-PAYLOAD".to_string(),
        );
        headers.insert("x-oss-date".to_string(), iso_datetime.to_string());

        // 添加额外的headers
        for (key, value) in additional_headers {
            headers.insert(key.to_lowercase(), value.trim().to_string());
        }

        let canonical_headers = self.build_canonical_headers(&headers);
        let signed_headers = self.build_signed_headers(&headers);

        // 3. 构建canonical_request
        // 格式：HTTPMethod\nURI\nQuery\nHeaders\nSignedHeaders\nPayload
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\nUNSIGNED-PAYLOAD",
            method, path, canonical_query_string, canonical_headers, signed_headers
        );

        // 4. 构建string_to_sign
        let credential_scope = format!("{}/{}/oss/aliyun_v4_request", date_only, region);
        let string_to_sign = format!(
            "OSS4-HMAC-SHA256\n{}\n{}\n{}",
            iso_datetime,
            credential_scope,
            digest(&canonical_request)
        );

        // 5. 计算签名
        let signing_key = self.get_v4_signing_key(date_only, &region)?;
        let signature = hmac_sha256(&signing_key, string_to_sign.as_bytes());
        let signature_hex = hex::encode(signature);

        // 6. 构建Authorization header - 使用AdditionalHeaders而不是signed_headers
        let additional_headers_str = self.build_additional_headers(&headers);
        let mut authorization = format!(
            "OSS4-HMAC-SHA256 Credential={}/{}, Signature={}",
            self.access_key_id, credential_scope, signature_hex
        );

        if !additional_headers_str.is_empty() {
            authorization.push_str(&format!(", AdditionalHeaders={}", additional_headers_str));
        }

        Ok(authorization)
    }

    fn extract_region_from_host(&self, host: &str) -> String {
        if host.contains("oss-") && host.contains(".aliyuncs.com") {
            if let Some(start) = host.find("oss-") {
                if let Some(end) = host.find(".aliyuncs.com") {
                    let region_part = &host[start + 4..end];
                    if !region_part.is_empty() && region_part != "oss" {
                        return region_part.to_string();
                    }
                }
            }
        }
        "cn-hangzhou".to_string() // 默认region
    }

    fn build_canonical_query_string(&self, query_string: &str) -> String {
        if query_string.is_empty() || query_string == "?" {
            return String::new();
        }

        let query_str = if query_string.starts_with('?') {
            &query_string[1..]
        } else {
            query_string
        };

        let mut params = BTreeMap::new();
        for param in query_str.split('&') {
            if param.is_empty() {
                continue;
            }
            
            if let Some(eq_pos) = param.find('=') {
                let key = &param[..eq_pos];
                let value = &param[eq_pos + 1..];
                
                // 直接使用RFC 3986标准进行URL编码，不进行解码
                let encoded_key = percent_encode(key.as_bytes(), QUERY).to_string();
                let encoded_value = percent_encode(value.as_bytes(), QUERY).to_string();
                params.insert(encoded_key, encoded_value);
            } else {
                // 没有值的参数
                let encoded_key = percent_encode(param.as_bytes(), QUERY).to_string();
                params.insert(encoded_key, String::new());
            }
        }

        params
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    k.clone()
                } else {
                    format!("{}={}", k, v)
                }
            })
            .collect::<Vec<_>>()
            .join("&")
    }

    fn build_canonical_headers(&self, headers: &BTreeMap<String, String>) -> String {
        headers
            .iter()
            .map(|(k, v)| format!("{}:{}", k.to_lowercase(), v.trim()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn build_signed_headers(&self, headers: &BTreeMap<String, String>) -> String {
        let mut sorted_keys: Vec<_> = headers.keys().map(|k| k.to_lowercase()).collect();
        sorted_keys.sort();
        sorted_keys.join(";")
    }

    /// 构建AdditionalHeaders - 和SignedHeaders内容一�?
    fn build_additional_headers(&self, headers: &BTreeMap<String, String>) -> String {
        self.build_signed_headers(headers)
    }

    fn get_v4_signing_key(&self, date: &str, region: &str) -> Result<Vec<u8>, OssError> {
        // OSS V4 签名密钥派生算法
        // kSecret = your secret access key
        // kDate = HMAC("aliyun_v4" + kSecret, Date)
        // kRegion = HMAC(kDate, Region) 
        // kService = HMAC(kRegion, Service)
        // kSigning = HMAC(kService, "aliyun_v4_request")
        
        let secret_key = format!("aliyun_v4{}", self.access_key_secret);
        let date_key = hmac_sha256(secret_key.as_bytes(), date.as_bytes());
        let region_key = hmac_sha256(&date_key, region.as_bytes());
        let service_key = hmac_sha256(&region_key, b"oss");
        let signing_key = hmac_sha256(&service_key, b"aliyun_v4_request");

        Ok(signing_key)
    }

    fn parse_buckets_xml(&self, xml: &str) -> Result<ListBucketsResponse, OssError> {
        use quick_xml::de::from_str;
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        struct ListAllMyBucketsResult {
            #[serde(rename = "Buckets")]
            buckets: BucketsWrapper,
        }

        #[derive(Debug, Deserialize)]
        struct BucketsWrapper {
            #[serde(rename = "Bucket")]
            bucket: Vec<BucketXml>,
        }

        #[derive(Debug, Deserialize)]
        struct BucketXml {
            #[serde(rename = "Name")]
            name: String,
            #[serde(rename = "Location")]
            location: String,
            #[serde(rename = "CreationDate")]
            creation_date: String,
            #[serde(rename = "StorageClass")]
            storage_class: String,
            #[serde(rename = "ExtranetEndpoint")]
            extranet_endpoint: String,
            #[serde(rename = "IntranetEndpoint")]
            intranet_endpoint: String,
        }

        let parsed: ListAllMyBucketsResult =
            from_str(xml).map_err(|e| OssError::XmlParsingFailed(e.to_string()))?;

        let buckets = parsed
            .buckets
            .bucket
            .into_iter()
            .map(|b| Bucket {
                name: b.name,
                location: b.location,
                creation_date: b.creation_date,
                storage_class: b.storage_class,
                extranet_endpoint: b.extranet_endpoint,
                intranet_endpoint: b.intranet_endpoint,
            })
            .collect();

        Ok(ListBucketsResponse { buckets })
    }

    fn parse_objects_xml(&self, xml: &str) -> Result<ListObjectsResponse, OssError> {
        use quick_xml::de::from_str;
        use serde::Deserialize;

        #[derive(Debug, Deserialize)]
        struct ListBucketResult {
            #[serde(rename = "IsTruncated")]
            is_truncated: bool,
            #[serde(rename = "NextContinuationToken")]
            next_continuation_token: Option<String>,
            #[serde(rename = "Contents")]
            contents: Option<Vec<ObjectXml>>,
        }

        #[derive(Debug, Deserialize)]
        struct ObjectXml {
            #[serde(rename = "Key")]
            key: String,
            #[serde(rename = "LastModified")]
            last_modified: String,
            #[serde(rename = "Size")]
            size: u64,
            #[serde(rename = "StorageClass")]
            storage_class: String,
        }

        let parsed: ListBucketResult =
            from_str(xml).map_err(|e| OssError::XmlParsingFailed(e.to_string()))?;

        let objects = parsed
            .contents
            .unwrap_or_default()
            .into_iter()
            .map(|obj| ObjectInfo {
                key: obj.key,
                last_modified: obj.last_modified,
                size: obj.size,
                storage_class: obj.storage_class,
            })
            .collect();

        Ok(ListObjectsResponse {
            objects,
            is_truncated: parsed.is_truncated,
            next_continuation_token: parsed.next_continuation_token,
        })
    }
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    Mac::update(&mut mac, data);
    mac.finalize().into_bytes().to_vec()
}



