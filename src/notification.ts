import NotificationApp from "./NotificationApp.svelte";
import { mount } from "svelte";

const app = mount(NotificationApp, { target: document.getElementById("app")! });
export default app;
