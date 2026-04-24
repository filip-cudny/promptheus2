import { mount } from "svelte";
import ShellToolbarApp from "./ShellToolbarApp.svelte";

const target = document.getElementById("app");
if (!target) throw new Error("missing #app root");

mount(ShellToolbarApp, { target });
