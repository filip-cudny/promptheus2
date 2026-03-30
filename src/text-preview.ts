import TextPreviewApp from "./TextPreviewApp.svelte";
import { mount } from "svelte";

const app = mount(TextPreviewApp, { target: document.getElementById("app")! });
export default app;
