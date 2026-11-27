// Menú contextual propio (sustituye al del navegador). Estado global: App.vue
// pinta un único <ContextMenu>; cualquier componente lo abre con `open`.
import { shallowRef } from "vue";

export interface MenuItem {
  label: string;
  action?: () => unknown;
  shortcut?: string;
  disabled?: boolean;
  danger?: boolean;
  /** Muestra una marca (opción activa o casilla). */
  checked?: boolean;
  children?: MenuEntry[];
}
export type MenuEntry = MenuItem | "separator";

export interface MenuState {
  x: number;
  y: number;
  items: MenuEntry[];
  returnFocus: HTMLElement | null;
}

const state = shallowRef<MenuState | null>(null);

/** Coordenadas del evento; con teclado (Mayús+F10) llegan a 0, así que se usa el elemento. */
function anchorPoint(event: MouseEvent): { x: number; y: number } {
  if (event.clientX || event.clientY) return { x: event.clientX, y: event.clientY };
  const rect = (event.target as Element | null)?.getBoundingClientRect();
  return rect ? { x: rect.left + 8, y: rect.bottom } : { x: 40, y: 40 };
}

export function useContextMenu() {
  function open(event: MouseEvent, items: MenuEntry[]) {
    event.preventDefault();
    event.stopPropagation();
    state.value = { ...anchorPoint(event), items, returnFocus: document.activeElement as HTMLElement | null };
  }
  function openAt(x: number, y: number, items: MenuEntry[]) {
    state.value = { x, y, items, returnFocus: document.activeElement as HTMLElement | null };
  }
  function close(restoreFocus = true) {
    const previous = state.value?.returnFocus;
    state.value = null;
    if (restoreFocus) previous?.focus?.();
  }
  return { state, open, openAt, close };
}
