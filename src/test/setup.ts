// Entorno DOM para `bun test`. happy-dom rompe DOMPurify (ver MIGRACION.md),
// así que se arma un `window`/`document` globales con jsdom en su lugar.
import { JSDOM } from "jsdom";

const dom = new JSDOM("<!doctype html><html><body></body></html>", { url: "http://localhost/" });

// Bun ya define `window`/`document`/… como propiedades de solo lectura; hay
// que reemplazarlas (no solo asignarlas) para que apunten a las de jsdom.
const define = (key: PropertyKey, value: unknown) => Object.defineProperty(globalThis, key, { value, configurable: true, writable: true, enumerable: true });

for (const key of Object.getOwnPropertyNames(dom.window)) {
  if (key in globalThis && key !== "window" && key !== "document" && key !== "navigator") continue;
  define(key, (dom.window as unknown as Record<string, unknown>)[key]);
}
define("window", dom.window);
define("document", dom.window.document);
define("navigator", dom.window.navigator);

// Puente mínimo de Tauri: solo lo que usa código "puro" bajo test (p. ej.
// `attachmentUrl`, que llama a `convertFileSrc`). Un `invoke()` real a un
// comando del backend no debe funcionar aquí: estos son tests unitarios, no
// de integración con `tauri dev`.
(dom.window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
  convertFileSrc: (filePath: string, protocol = "asset") => `${protocol}://${filePath}`,
  invoke: () => {
    throw new Error("invoke() de Tauri no está disponible en los tests unitarios (bun test)");
  },
};
