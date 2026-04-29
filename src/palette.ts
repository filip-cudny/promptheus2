import PaletteApp from "./PaletteApp.svelte";
import { mount } from "svelte";

const app = mount(PaletteApp, { target: document.getElementById("app")! });
export default app;
