<script lang="ts">
  import { PanelLeft, SquarePen } from "lucide-svelte";
  import { ICON_SIZE } from "$lib/constants/ui";

  let {
    sidebarOpen = false,
    onToggleSidebar,
    onNewChat,
  }: {
    sidebarOpen?: boolean;
    onToggleSidebar: () => void;
    onNewChat: () => void;
  } = $props();
</script>

<div class="top-buttons" class:sidebar-open={sidebarOpen}>
  <button
    class="top-btn sidebar-toggle"
    class:hidden={sidebarOpen}
    onclick={onToggleSidebar}
    title="Toggle conversations"
  >
    <PanelLeft size={ICON_SIZE.md} />
  </button>
  <button class="top-btn" onclick={onNewChat} title="New conversation">
    <SquarePen size={ICON_SIZE.md} />
  </button>
</div>

<style>
  .top-buttons {
    position: absolute;
    top: 6px;
    left: 6px;
    z-index: 201;
    display: flex;
    gap: var(--space-2);
    transition: transform var(--motion-slow) var(--ease-default);
  }

  .top-buttons.sidebar-open {
    transform: translateX(240px);
  }

  .sidebar-toggle {
    width: 28px;
    overflow: visible;
    transition: width var(--motion-slow) var(--ease-default), opacity var(--motion-slow) var(--ease-default);
  }

  .sidebar-toggle.hidden {
    width: 0;
    opacity: 0;
    overflow: hidden;
    pointer-events: none;
  }

  .top-btn {
    width: 28px;
    height: 28px;
    border-radius: var(--radius-lg);
    border: none;
    background: rgba(255, 255, 255, 0.03);
    backdrop-filter: blur(6px);
    -webkit-backdrop-filter: blur(6px);
    color: var(--text-disabled);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: var(--space-0);
    position: relative;
  }

  :global([data-platform="linux"]) .top-btn {
    background: var(--surface-overlay-faint);
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }

  .top-btn:hover {
    color: var(--text-secondary);
    background: var(--surface-overlay);
  }
</style>
