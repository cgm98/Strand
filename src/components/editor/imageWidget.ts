// Monta ImageBlock.vue dentro de CodeMirror (mismo patrón que tableWidget.ts).
// El componente sobrevive a los cambios del documento: solo se actualizan su
// Markdown y su posición, así no se pierden el formulario ni el visor abiertos.
import { createApp, h, shallowRef, type App, type ShallowRef } from "vue";
import { WidgetType, type EditorView } from "@codemirror/view";
import { Transaction } from "@codemirror/state";
import { isolateHistory } from "@codemirror/commands";
import ImageBlock from "./ImageBlock.vue";

interface MountedImage {
  app: App;
  source: ShallowRef<string>;
  from: number;
  to: number;
  editingSource: boolean;
}
const mounted = new WeakMap<HTMLElement, MountedImage>();
// Cada acción del contenedor es un paso propio de deshacer (CodeMirror agrupa
// los cambios seguidos de menos de 500 ms).
const userEvent = (name: string) => [Transaction.userEvent.of(name), isolateHistory.of("full")];

export class ImageWidget extends WidgetType {
  constructor(
    readonly source: string,
    readonly from: number,
    readonly to: number,
    /** true cuando el cursor está en el Markdown: el contenedor va debajo. */
    readonly editingSource: boolean,
  ) {
    super();
  }

  eq(other: ImageWidget) {
    return other.source === this.source && other.from === this.from && other.to === this.to && other.editingSource === this.editingSource;
  }

  toDOM(view: EditorView) {
    const dom = document.createElement("span");
    dom.className = "cm-image-host" + (this.editingSource ? " editing-source" : "");
    const record = { source: shallowRef(this.source), from: this.from, to: this.to, editingSource: this.editingSource } as MountedImage;

    const replace = (insert: string) => view.dispatch({ changes: { from: record.from, to: record.to, insert }, annotations: userEvent("input.image") });

    const remove = () => {
      const line = view.state.doc.lineAt(record.from);
      let { from, to } = record;
      // Si la imagen ocupa la línea entera, quitar también la línea.
      if (line.text.trim() === view.state.sliceDoc(record.from, record.to).trim()) {
        from = line.from;
        to = line.to;
        if (line.number < view.state.doc.lines) to += view.state.lineBreak.length;
        else if (line.number > 1) from -= view.state.lineBreak.length;
      }
      view.dispatch({ changes: { from, to }, selection: { anchor: from }, annotations: userEvent("delete.image") });
      view.focus();
    };

    const insertAfter = (markdowns: string[]) => {
      const end = view.state.doc.lineAt(record.to).to;
      const nl = view.state.lineBreak;
      const insert = nl + markdowns.join(nl);
      view.dispatch({ changes: { from: end, insert }, selection: { anchor: end + insert.length }, annotations: userEvent("input.image") });
    };

    const editSource = () => {
      view.dispatch({ selection: { anchor: record.to }, scrollIntoView: true });
      view.focus();
    };

    record.app = createApp({
      render: () =>
        h(ImageBlock, {
          source: record.source.value,
          editingSource: record.editingSource,
          onUpdate: replace,
          onRemove: remove,
          onInsertAfter: insertAfter,
          onEditSource: editSource,
        }),
    });
    mounted.set(dom, record);
    record.app.mount(dom);
    return dom;
  }

  updateDOM(dom: HTMLElement) {
    const record = mounted.get(dom);
    if (!record || record.editingSource !== this.editingSource) return false;
    record.from = this.from;
    record.to = this.to;
    record.source.value = this.source;
    return true;
  }

  destroy(dom: HTMLElement) {
    mounted.get(dom)?.app.unmount();
    mounted.delete(dom);
  }

  ignoreEvent() {
    return true;
  }
}
