import SettingsDialogApp from "./SettingsDialogApp.svelte";
import { mount } from "svelte";

const app = mount(SettingsDialogApp, { target: document.getElementById("app")! });
export default app;
