import ProviderMenuApp from "./ProviderMenuApp.svelte";
import { mount } from "svelte";

const app = mount(ProviderMenuApp, { target: document.getElementById("app")! });
export default app;
