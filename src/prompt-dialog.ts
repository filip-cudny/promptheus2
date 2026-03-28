import "highlight.js/styles/night-owl.min.css";
import PromptDialogApp from "./PromptDialogApp.svelte";
import { mount } from "svelte";

const app = mount(PromptDialogApp, { target: document.getElementById("app")! });
export default app;
