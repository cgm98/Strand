import DOMPurify from "dompurify";

// DOMPurify solo deja pasar esquemas conocidos (http, mailto…). Añadimos
// `attachment:` para que sobrevivan las imágenes servidas por el protocolo propio.
const ALLOWED_URI_REGEXP = /^(?:(?:(?:f|ht)tps?|mailto|tel|attachment|asset):|[^a-z]|[a-z+.-]+(?:[^a-z+.\-:]|$))/i;

export function sanitizeHtml(html: string): string {
  return DOMPurify.sanitize(html, { ALLOWED_URI_REGEXP });
}

export function sanitizeSvg(svg: string): string {
  return DOMPurify.sanitize(svg, { USE_PROFILES: { svg: true, svgFilters: true } });
}
