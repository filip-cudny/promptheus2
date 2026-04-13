import { attachConsole } from "@tauri-apps/plugin-log";

await attachConsole();

import "highlight.js/styles/night-owl.min.css";

import ConversationDialogApp from "./ConversationDialogApp.svelte";
import { mount } from "svelte";

const app = mount(ConversationDialogApp, { target: document.getElementById("app")! });
export default app;
