// API 配置
export const API_CONFIG = {
  // 开发环境使用代理，生产环境直接访问前端域名的API路由
  BASE_URL: import.meta.env.PROD
    ? "https://gurl.honahec.cc"
    : "http://localhost:8003",
  // 下载链接基础URL（后端会直接返回完整的下载URL）
  DOWNLOAD_BASE: "https://api.honahec.cc",
  // 阿里云OSS默认Endpoint
  DEFAULT_ALIYUN_DEFAULT_ENDPOINT:
    import.meta.env.VITE_ALIYUN_DEFAULT_ENDPOINT ||
    "oss-cn-shanghai.aliyuncs.com",
  DEFAULT_ALIYUN_DEFAULT_BUCKET: import.meta.env.VITE_ALIYUN_DEFAULT_BUCKET,
};

// OAuth2 配置
export const OAUTH_CONFIG = {
  // OAuth2 客户端 ID (必需，从环境变量读取)
  CLIENT_ID: import.meta.env.VITE_OAUTH_CLIENT_ID,
  // OAuth2 授权端点
  AUTHORIZE_URL:
    import.meta.env.VITE_OAUTH_AUTHORIZE_URL ||
    "https://sso.honahec.cc/oauth/authorize",
  // OAuth2 回调地址
  REDIRECT_URI:
    import.meta.env.VITE_OAUTH_REDIRECT_URI ||
    (import.meta.env.PROD
      ? "https://gurl.honahec.cc"
      : "http://localhost:5173"),
  // OAuth2 请求的权限范围
  SCOPE: "username permissions",
};

export default API_CONFIG;
