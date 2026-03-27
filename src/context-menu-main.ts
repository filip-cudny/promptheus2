import ContextMenuApp from "./ContextMenuApp.svelte";
import { mount } from "svelte";

const app = mount(ContextMenuApp, {
  target: document.getElementById("app")!,
});

export default app;
