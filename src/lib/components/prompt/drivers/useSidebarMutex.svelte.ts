export function useSidebarMutex<T>() {
  let menuOpenId = $state<string | null>(null);
  let editingId = $state<string | null>(null);
  let confirmDelete = $state<T | null>(null);

  function closeMenu() {
    menuOpenId = null;
  }

  function clearEditing() {
    editingId = null;
  }

  function clearConfirm() {
    confirmDelete = null;
  }

  function closeAll() {
    closeMenu();
    clearEditing();
    clearConfirm();
  }

  return {
    get menuOpenId() {
      return menuOpenId;
    },
    get editingId() {
      return editingId;
    },
    get confirmDelete() {
      return confirmDelete;
    },
    openMenu(id: string) {
      const same = menuOpenId === id;
      closeAll();
      if (!same) menuOpenId = id;
    },
    startRename(id: string) {
      closeAll();
      editingId = id;
    },
    startDelete(item: T) {
      closeAll();
      confirmDelete = item;
    },
    closeMenu,
    clearEditing,
    clearConfirm,
    closeAll,
  };
}

export type SidebarMutex<T> = ReturnType<typeof useSidebarMutex<T>>;
