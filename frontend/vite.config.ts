import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      // 开发环境代理API路由到后端
      "/login": "http://localhost:8003",
      "/sign": "http://localhost:8003",
      "/links": "http://localhost:8003",
      "/download": "http://localhost:8003",
    },
  },
  build: {
    outDir: "dist",
  },
  define: {
    // 生产环境API地址
    __API_BASE_URL__: JSON.stringify("https://api.honahec.cc"),
  },
});
