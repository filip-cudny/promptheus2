import ContextEditorApp from "./ContextEditorApp.svelte";
import { mount } from "svelte";

const app = mount(ContextEditorApp, { target: document.getElementById("app")! });
export default app;
