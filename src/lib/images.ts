// Imágenes en Markdown: leer y escribir `![alt](src "título")`, resolver su URL
// y subir archivos como adjuntos. Lo usan el editor, el contenedor de imagen y
// la vista de lectura.
import { api, ApiError, attachmentRef, attachmentUrl } from "./api";

export interface ImageRef {
  alt: string;
  src: string;
  title?: string;
}

const IMAGE_RE = /^!\[([^\]\n]*)\]\(\s*(?:<([^>\n]+)>|([^\s)]+))(?:\s+"([^"\n]*)")?\s*\)$/;

export function parseImage(markdown: string): ImageRef | null {
  const m = IMAGE_RE.exec(markdown.trim());
  if (!m) return null;
  return { alt: m[1], src: m[2] ?? m[3], title: m[4] || undefined };
}

export function serializeImage({ alt, src, title }: ImageRef): string {
  const cleanAlt = alt.replace(/[[\]\n]/g, "").trim();
  const cleanSrc = src.trim();
  const target = /[\s()]/.test(cleanSrc) ? `<${cleanSrc}>` : cleanSrc;
  const cleanTitle = title?.replace(/["\n]/g, "").trim();
  return `![${cleanAlt}](${target}${cleanTitle ? ` "${cleanTitle}"` : ""})`;
}

/** URL que puede cargar el webview, o null si el esquema no es seguro. */
export function imageUrl(src: string): string | null {
  if (src.startsWith("attachments/")) return attachmentUrl(src);
  if (/^https?:\/\//i.test(src) || /^data:image\/(png|jpe?g|gif|webp);/i.test(src)) return src;
  return null;
}

/** Texto alternativo por defecto a partir del nombre de archivo. */
export function altFromFileName(name: string): string {
  return (
    name
      .replace(/\.[^.]+$/, "")
      .replace(/[[\]_-]+/g, " ")
      .trim() || "imagen"
  );
}

export const isImageFile = (f: File) => f.type.startsWith("image/");

/**
 * Guarda cada imagen como adjunto y devuelve sus referencias. Los fallos se
 * notifican uno a uno con `onError` y no detienen al resto.
 */
export async function uploadImages(files: File[], onError: (text: string) => void): Promise<ImageRef[]> {
  const refs: ImageRef[] = [];
  for (const file of files.filter(isImageFile)) {
    try {
      const name = await api.saveAttachment(new Uint8Array(await file.arrayBuffer()));
      refs.push({ alt: altFromFileName(file.name), src: attachmentRef(name) });
    } catch (e) {
      onError(`No se pudo guardar «${file.name}»: ${e instanceof ApiError ? e.message : String(e)}`);
    }
  }
  return refs;
}

/** Abre el selector de archivos del sistema y devuelve las imágenes elegidas. */
export function pickImageFiles(multiple = true): Promise<File[]> {
  return new Promise((resolve) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "image/png,image/jpeg,image/gif,image/webp";
    input.multiple = multiple;
    input.onchange = () => resolve(Array.from(input.files ?? []));
    input.oncancel = () => resolve([]);
    input.click();
  });
}
