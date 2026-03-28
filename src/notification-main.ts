import { attachConsole } from "@tauri-apps/plugin-log";
import NotificationWindowApp from "./NotificationWindowApp.svelte";
import { mount } from "svelte";

await attachConsole();

const app = mount(NotificationWindowApp, {
  target: document.getElementById("app")!,
});

export default app;
