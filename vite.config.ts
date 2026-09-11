import { defineConfig } from "vite";
import path from "node:path";
import { readFileSync } from "node:fs";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// Single source of truth for the app version: src-tauri/Cargo.toml, injected
// as a build-time constant (no `import.meta.env`, per repo rules).
const cargoVersion =
    readFileSync(path.resolve(import.meta.dirname, "src-tauri/Cargo.toml"), "utf8").match(
        /^version\s*=\s*"([^"]+)"/m
    )?.[1] ?? "0.0.0";

// https://vite.dev/config/
export default defineConfig(() => ({
    define: {
        __APP_VERSION__: JSON.stringify(cargoVersion),
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
