// Portado de cogni-vault sin cambios.
import { createApp, shallowRef, h, type App, type ShallowRef } from "vue";
import { WidgetType, type EditorView } from "@codemirror/view";
import { Transaction } from "@codemirror/state";
import { undo, redo } from "@codemirror/commands";
import TableEditor from "./TableEditor.vue";

interface MountedTable {
  app: App;
  source: ShallowRef<string>;
  from: number;
  to: number;
}
const mounted = new WeakMap<HTMLElement, MountedTable>();

export class TableWidget extends WidgetType {
  constructor(
    readonly source: string,
    readonly from: number,
    readonly to: number,
  ) {
    super();
  }
  eq(other: TableWidget) {
    return this.source === other.source && this.from === other.from && this.to === other.to;
  }
  toDOM(view: EditorView) {
    const dom = document.createElement("div");
    const record = { source: shallowRef(this.source), from: this.from, to: this.to } as MountedTable;
    record.app = createApp({
      render: () =>
        h(TableEditor, {
          source: record.source.value,
          onChange: (source: string) => {
            const newline = view.state.lineBreak;
            // La tabla puede estar indentada por estar dentro de una lista o una cita; esa
            // indentación no viene en `source` (siempre lo serializa `markdownTable.ts` sin
            // ella), así que hay que reponerla en cada línea o la tabla se saldría de la lista.
            const indent = /^[ \t]*/.exec(record.source.value)?.[0] ?? "";
            const indented = indent
              ? source
                  .split("\n")
                  .map((line) => indent + line)
                  .join("\n")
              : source;
            const insert = indented.replace(/\n/g, newline);
            view.dispatch({ changes: { from: record.from, to: record.to, insert }, annotations: Transaction.userEvent.of("input.table") });
          },
          onUndo: () => undo(view),
          onRedo: () => redo(view),
        }),
    });
    mounted.set(dom, record);
    record.app.mount(dom);
    return dom;
  }
  updateDOM(dom: HTMLElement) {
    const record = mounted.get(dom);
    if (!record) return false;
    record.from = this.from;
    record.to = this.to;
    record.source.value = this.source;
    return true; // Keep the cell's DOM and cursor while its Markdown is updated.
  }
  destroy(dom: HTMLElement) {
    mounted.get(dom)?.app.unmount();
    mounted.delete(dom);
  }
  ignoreEvent() {
    return true;
  }
}
