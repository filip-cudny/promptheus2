import { attachConsole } from "@tauri-apps/plugin-log";
import ContextMenuApp from "./ContextMenuApp.svelte";
import { mount } from "svelte";

await attachConsole();

const app = mount(ContextMenuApp, {
  target: document.getElementById("app")!,
});

export default app;
