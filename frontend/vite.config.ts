import { defineConfig } from "vite";

import react from "@vitejs/plugin-react-swc";
import svgr from "@svgr/rollup";

// https://vitejs.dev/config/
export default defineConfig({
  base: "/",
  plugins: [react(), svgr()],
  build: {
    chunkSizeWarningLimit: 6000,
  },
  server: {
    proxy: {
      "/api": {
        target: "http://localhost:3000",
        changeOrigin: true,
      },
    },
  },
});
