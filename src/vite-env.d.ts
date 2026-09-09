declare module "*.svg" {
    const src: string;
    export default src;
}

declare module "*.svg?raw" {
    const markup: string;
    export default markup;
}

declare module "*.css";
