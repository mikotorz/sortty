import { defineConfig } from "vitest/config";
import path from "node:path";

// A minimal, standalone config for the frontend unit test suite — deliberately
// not reusing vite.config.js's tauri-dev-server settings or the sveltekit()/
// tailwindcss() plugins, since these tests are plain TypeScript (no .svelte
// rendering yet) and don't need either.
export default defineConfig({
  resolve: {
    alias: {
      $lib: path.resolve(import.meta.dirname, "./src/lib"),
    },
  },
  test: {
    include: ["src/**/*.test.ts"],
  },
});
