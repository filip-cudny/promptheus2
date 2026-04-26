declare module "*.svg?raw" {
  const content: string;
  export default content;
}

declare module "*.png?url" {
  const url: string;
  export default url;
}
