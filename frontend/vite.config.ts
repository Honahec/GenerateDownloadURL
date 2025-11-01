import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // Set envDir to parent directory, let Vite read .env file from root directory
  envDir: "../",
  server: {
    port: 5173,
    proxy: {
      // Proxy API routes to backend in development environment
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
    // Production environment API address
    __API_BASE_URL__: JSON.stringify("https://api.honahec.cc"),
  },
});
