import HistoryDialogApp from "./HistoryDialogApp.svelte";
import { mount } from "svelte";

const app = mount(HistoryDialogApp, { target: document.getElementById("app")! });
export default app;
