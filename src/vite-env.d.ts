declare module "*.svg" {
    const src: string;
    export default src;
}

declare module "*.svg?raw" {
    const markup: string;
    export default markup;
}

/** Injected by Vite `define` from package.json `version` (vite.config.ts). */
declare const __APP_VERSION__: string;

declare module "*.css";
