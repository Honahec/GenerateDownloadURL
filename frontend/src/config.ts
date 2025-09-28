// API 配置
export const API_CONFIG = {
  // 开发环境使用代理，生产环境直接访问前端域名的API路由
  BASE_URL: import.meta.env.PROD
    ? "https://gurl.honahec.cc"
    : "http://localhost:8003",
  // 下载链接基础URL（后端会直接返回完整的下载URL）
  DOWNLOAD_BASE: "https://api.honahec.cc",
};

export default API_CONFIG;
