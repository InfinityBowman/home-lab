import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");

  return {
    plugins: [react(), tailwindcss()],
    resolve: {
      alias: {
        "@": path.resolve(__dirname, "./src"),
      },
    },
    server: {
      proxy: {
        "/api": {
          target: env.API_URL || "http://192.168.1.100:5170",
          changeOrigin: true,
          headers: env.API_SECRET
            ? { Authorization: `Bearer ${env.API_SECRET}` }
            : undefined,
        },
      },
    },
  };
});
