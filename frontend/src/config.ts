// API Configuration
export const API_CONFIG = {
  // Use proxy in development, directly access frontend domain API routes in production
  BASE_URL: import.meta.env.PROD
    ? "https://gurl.honahec.cc"
    : "http://localhost:8003",
  // Download link base URL (backend will return complete download URL)
  DOWNLOAD_BASE: "https://api.honahec.cc",
  // Aliyun OSS default endpoint
  DEFAULT_ALIYUN_DEFAULT_ENDPOINT:
    import.meta.env.VITE_ALIYUN_DEFAULT_ENDPOINT ||
    "oss-cn-shanghai.aliyuncs.com",
  DEFAULT_ALIYUN_DEFAULT_BUCKET: import.meta.env.VITE_ALIYUN_DEFAULT_BUCKET,
};

// OAuth2 Configuration
export const OAUTH_CONFIG = {
  // OAuth2 Client ID (Required, read from environment variable)
  CLIENT_ID: import.meta.env.VITE_OAUTH_CLIENT_ID,
  // OAuth2 Authorization endpoint
  AUTHORIZE_URL:
    import.meta.env.VITE_OAUTH_AUTHORIZE_URL ||
    "https://sso.honahec.cc/oauth/authorize",
  // OAuth2 Redirect URI
  REDIRECT_URI:
    import.meta.env.VITE_OAUTH_REDIRECT_URI ||
    (import.meta.env.PROD
      ? "https://gurl.honahec.cc"
      : "http://localhost:5173"),
  // OAuth2 Requested scopes
  SCOPE: "username permissions",
};

export default API_CONFIG;
