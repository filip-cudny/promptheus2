import ImagePreviewApp from "./ImagePreviewApp.svelte";
import { mount } from "svelte";

const app = mount(ImagePreviewApp, { target: document.getElementById("app")! });
export default app;
