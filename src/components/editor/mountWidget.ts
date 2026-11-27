// Punto único donde se monta un componente Vue dentro de un nodo que gestiona
// CodeMirror (widgets de bloque en liveDecorations.ts). Centralizarlo evita
// tener más de una llamada a `createApp` por archivo (vue/one-component-per-file)
// y comparte la misma forma de limpiar la app al destruir el widget.
import { createApp, h, type App, type Component } from "vue";

const apps = new WeakMap<HTMLElement, App>();

export function mountWidget(target: HTMLElement, component: Component, props: Record<string, unknown>) {
  const app = createApp({ render: () => h(component, props) });
  app.mount(target);
  apps.set(target, app);
}

export function unmountWidget(target: HTMLElement) {
  apps.get(target)?.unmount();
  apps.delete(target);
}
