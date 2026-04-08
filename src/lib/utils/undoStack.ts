export interface UndoEntry {
  text: string;
  cursorStart: number;
  cursorEnd: number;
}

export class UndoStack {
  private stack: UndoEntry[] = [];
  private pointer: number = -1;
  private maxSize: number;

  constructor(maxSize = 100) {
    this.maxSize = maxSize;
  }

  push(entry: UndoEntry): void {
    this.stack.length = this.pointer + 1;
    this.stack.push(entry);
    if (this.stack.length > this.maxSize) {
      this.stack.shift();
    }
    this.pointer = this.stack.length - 1;
  }

  undo(): UndoEntry | null {
    if (this.pointer <= 0) return null;
    this.pointer--;
    return this.stack[this.pointer];
  }

  redo(): UndoEntry | null {
    if (this.pointer >= this.stack.length - 1) return null;
    this.pointer++;
    return this.stack[this.pointer];
  }

  reset(initial?: UndoEntry): void {
    this.stack = initial ? [initial] : [];
    this.pointer = initial ? 0 : -1;
  }
}
