import { defineConfig } from "vite";

export default defineConfig({
  root: "src/client",
  build: { outDir: "../../dist", emptyOutDir: true },
  // "/api/" with the trailing slash, otherwise "/api.ts" is proxied too.
  server: { port: 4000, proxy: { "/api/": "http://localhost:4001" } },
});
