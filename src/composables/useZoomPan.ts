// Zoom/pan propio (sin dependencias) para los visores a pantalla completa
// (Mermaid ampliado, visor de imágenes): rueda del mouse, arrastre con el
// puntero, doble clic y teclado (+/-/0). Cada llamada crea un estado nuevo,
// así que cada visor tiene su propio zoom independiente.
//
// El elemento al que se le aplica `transformStyle()` DEBE tener
// `transform-origin: 0 0`. Con el origen por defecto (centro), acercar/alejar
// desplaza la imagen por una cantidad que depende del tamaño del elemento
// (`transform-origin` reescala también la traslación ya aplicada), así que al
// alejar después de haber arrastrado la imagen "se escapaba" de la vista. Con
// el origen en la esquina, la traslación es un desplazamiento en píxeles de
// pantalla puro e independiente del zoom, y el zoom se ancla a mano (al centro
// del elemento con los botones/teclado, al cursor con la rueda) usando su
// posición real en pantalla (`getBoundingClientRect`), así el punto bajo el
// cursor/centro no se mueve al cambiar la escala.
import { reactive, type Ref } from "vue";

export interface ZoomPanOptions {
  min?: number;
  max?: number;
  step?: number;
}

export function useZoomPan(elRef: Ref<HTMLElement | undefined>, options: ZoomPanOptions = {}) {
  const min = options.min ?? 1;
  const max = options.max ?? 6;
  const step = options.step ?? 1.3;
  const state = reactive({ scale: 1, x: 0, y: 0, dragging: false });
  let pointerId: number | null = null;
  let start = { x: 0, y: 0, panX: 0, panY: 0 };

  const clamp = (value: number) => Math.min(max, Math.max(min, value));

  function elementCenter() {
    const rect = elRef.value?.getBoundingClientRect();
    return rect ? { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 } : { x: 0, y: 0 };
  }
  // Cambia la escala manteniendo fijo el punto de pantalla `anchor` (el centro
  // del elemento para los botones/teclado, el cursor para la rueda).
  function anchoredZoom(nextScale: number, anchor: { x: number; y: number }) {
    const clamped = clamp(nextScale);
    if (clamped !== state.scale) {
      const rect = elRef.value?.getBoundingClientRect();
      if (rect && rect.width > 0 && rect.height > 0) {
        const ratio = 1 - clamped / state.scale;
        state.x += (anchor.x - rect.left) * ratio;
        state.y += (anchor.y - rect.top) * ratio;
      }
      state.scale = clamped;
    }
    if (state.scale <= min) {
      state.x = 0;
      state.y = 0;
    }
  }
  function zoomIn() {
    anchoredZoom(state.scale * step, elementCenter());
  }
  function zoomOut() {
    anchoredZoom(state.scale / step, elementCenter());
  }
  function reset() {
    state.scale = 1;
    state.x = 0;
    state.y = 0;
  }
  function onWheel(event: WheelEvent) {
    event.preventDefault();
    anchoredZoom(state.scale * (event.deltaY < 0 ? step : 1 / step), { x: event.clientX, y: event.clientY });
  }
  function onPointerDown(event: PointerEvent) {
    if (state.scale <= min || event.button !== 0) return;
    pointerId = event.pointerId;
    start = { x: event.clientX, y: event.clientY, panX: state.x, panY: state.y };
    state.dragging = true;
    (event.currentTarget as HTMLElement).setPointerCapture(pointerId);
  }
  function onPointerMove(event: PointerEvent) {
    if (pointerId === null || event.pointerId !== pointerId) return;
    state.x = start.panX + (event.clientX - start.x);
    state.y = start.panY + (event.clientY - start.y);
  }
  function onPointerUp(event: PointerEvent) {
    if (pointerId === null || event.pointerId !== pointerId) return;
    pointerId = null;
    state.dragging = false;
  }
  function onDoubleClick(event: MouseEvent) {
    if (state.scale > min) reset();
    else anchoredZoom(2, { x: event.clientX, y: event.clientY });
  }
  function onKeydown(event: KeyboardEvent) {
    if (event.target instanceof HTMLElement && ["INPUT", "TEXTAREA"].includes(event.target.tagName)) return;
    if (event.key === "+" || event.key === "=") {
      event.preventDefault();
      zoomIn();
    } else if (event.key === "-") {
      event.preventDefault();
      zoomOut();
    } else if (event.key === "0") {
      event.preventDefault();
      reset();
    }
  }
  function transformStyle() {
    return {
      transform: `translate(${state.x}px, ${state.y}px) scale(${state.scale})`,
      transformOrigin: "0 0",
      cursor: state.scale > min ? (state.dragging ? "grabbing" : "grab") : "default",
    };
  }

  return { state, zoomIn, zoomOut, reset, onWheel, onPointerDown, onPointerMove, onPointerUp, onDoubleClick, onKeydown, transformStyle };
}
