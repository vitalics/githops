/// <reference types="vite/client" />

declare module "*.md?raw" {
  const content: string;
  export default content;
}

declare module "@docs/*" {
  const content: string;
  export default content;
}
