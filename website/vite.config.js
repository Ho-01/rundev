import { defineConfig } from "vite";

export default defineConfig(({ command }) => ({
  root: "src",
  base: command === "serve" ? "/" : process.env.SITE_BASE_PATH ?? "/rundev/",
  publicDir: "../public",
  build: {
    outDir: "../dist",
    emptyOutDir: true
  }
}));
