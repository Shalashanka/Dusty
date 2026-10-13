import type { Component } from "svelte";

export interface MenuItem {
  label?: string;
  icon?: Component<any>;
  danger?: boolean;
  disabled?: boolean;
  hint?: string;
  run?: () => void | Promise<void>;
  divider?: boolean;
  /** Small caption row. */
  caption?: string;
}

class MenuStore {
  open = $state(false);
  x = $state(0);
  y = $state(0);
  items = $state<MenuItem[]>([]);

  show(e: MouseEvent | { clientX: number; clientY: number }, items: MenuItem[]) {
    if ("preventDefault" in e) e.preventDefault();
    this.items = items;
    this.x = e.clientX;
    this.y = e.clientY;
    this.open = true;
  }

  close() {
    this.open = false;
  }
}

export const menu = new MenuStore();
