import { defineConfig } from "vite";
import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import pkg from "./package.json" with { type: "json" };
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
    // Single source of truth for the app version: package.json, injected as
    // a build-time constant (no `import.meta.env`, per repo rules).
    define: {
        __APP_VERSION__: JSON.stringify(pkg.version),
    },

    plugins: [react(), tailwindcss()],

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
        port: 1420,
        strictPort: true,
        host: host || false,
        hmr: host
            ? {
                  protocol: "ws",
                  host,
                  port: 1421,
              }
            : undefined,
        watch: {
            // 3. tell Vite to ignore watching `src-tauri`
            ignored: ["**/src-tauri/**"],
        },
    },
    resolve: {
        alias: {
            // `import.meta.dirname`: the native config loader does not support `__dirname`.
            "@": path.resolve(import.meta.dirname, "./src"),
        },
    },
}));
