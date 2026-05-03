const SHIFTED_CHAR_TO_DIGIT: Record<string, string> = {
  "!": "1", "@": "2", "#": "3", "$": "4", "%": "5",
  "^": "6", "&": "7", "*": "8", "(": "9", ")": "0",
};

type Opts = {
  isVisible: () => boolean;
  hasOpenPanel: () => boolean;
  closePanels: () => void;
  closeMenu: () => Promise<void> | void;
  moveSelection: (d: 1 | -1) => void;
  executeSelected: (shift: boolean) => Promise<void> | void;
  handleNumberInput: (digit: string, alt: boolean) => void;
};

export function useMenuKeyboard(opts: Opts) {
  let shiftHeld = $state(false);

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Shift") shiftHeld = true;
    if (!opts.isVisible()) return;

    switch (e.key) {
      case "Escape":
        e.preventDefault();
        if (opts.hasOpenPanel()) {
          opts.closePanels();
        } else {
          void opts.closeMenu();
        }
        break;
      case "ArrowDown":
        e.preventDefault();
        opts.moveSelection(1);
        break;
      case "ArrowUp":
        e.preventDefault();
        opts.moveSelection(-1);
        break;
      case "Enter":
        e.preventDefault();
        void opts.executeSelected(e.shiftKey);
        break;
      default: {
        if (e.key >= "0" && e.key <= "9") {
          e.preventDefault();
          opts.handleNumberInput(e.key, e.shiftKey);
          return;
        }
        const mappedDigit = SHIFTED_CHAR_TO_DIGIT[e.key];
        if (mappedDigit) {
          e.preventDefault();
          opts.handleNumberInput(mappedDigit, true);
        }
      }
    }
  }

  function onkeyup(e: KeyboardEvent) {
    if (e.key === "Shift") shiftHeld = false;
  }

  return {
    get shiftHeld() { return shiftHeld; },
    onkeydown,
    onkeyup,
  };
}

export type MenuKeyboard = ReturnType<typeof useMenuKeyboard>;
