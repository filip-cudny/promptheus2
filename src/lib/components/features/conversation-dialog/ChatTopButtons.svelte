<script lang="ts">
  import { PanelLeft, SquarePen } from "lucide-svelte";
  import ActionIconButton from "$lib/components/shared/ui/ActionIconButton.svelte";

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
  <div class="sidebar-toggle" class:hidden={sidebarOpen}>
    <ActionIconButton icon={PanelLeft} size="lg" label="Toggle conversation list" onclick={onToggleSidebar} />
  </div>
  <ActionIconButton icon={SquarePen} size="lg" label="New conversation" onclick={onNewChat} />
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
    display: flex;
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
</style>
