// Genera el CSS de un tema creado desde Apariencia → «Crear tema» (ver
// CreateThemeDialog.vue). El usuario solo elige los ~12 tokens "esenciales"
// (escala neutra: bg/surface/surface-2/border/muted/fg + acentos: primary/
// link/focus/success/warning/danger); el resto de variables que usan los
// temas incluidos (on-primary, primary-soft, mark, code-fg, folder, shadow,
// hl-*) se derivan aquí para que el tema no quede roto. Quien quiera afinarlas
// a mano puede abrir la carpeta del tema, igual que con uno duplicado.
export interface EssentialColors {
  bg: string;
  surface: string;
  surfaceAlt: string;
  border: string;
  muted: string;
  fg: string;
  primary: string;
  link: string;
  focus: string;
  success: string;
  warning: string;
  danger: string;
}

/** Punto de partida del formulario: los mismos valores que el tema predeterminado (base.css). */
export const DEFAULT_LIGHT: EssentialColors = {
  bg: "#f9f6f4",
  surface: "#efeae7",
  surfaceAlt: "#e0d7d1",
  border: "#c4b4ab",
  muted: "#665952",
  fg: "#161311",
  primary: "#b74506",
  link: "#ab3503",
  focus: "#d75a21",
  success: "#137738",
  warning: "#866405",
  danger: "#b4144e",
};

export const ESSENTIAL_FIELDS: Array<{ key: keyof EssentialColors; label: string }> = [
  { key: "bg", label: "Fondo" },
  { key: "surface", label: "Superficie" },
  { key: "surfaceAlt", label: "Superficie elevada" },
  { key: "border", label: "Borde" },
  { key: "muted", label: "Texto apagado" },
  { key: "fg", label: "Texto" },
  { key: "primary", label: "Acento" },
  { key: "link", label: "Enlaces" },
  { key: "focus", label: "Foco" },
  { key: "success", label: "Éxito" },
  { key: "warning", label: "Aviso" },
  { key: "danger", label: "Peligro" },
];

// ---------- Color: hex <-> rgb <-> hsl ----------

const clamp = (n: number, min: number, max: number) => Math.min(max, Math.max(min, n));

export function isValidHex(value: string): boolean {
  return /^#?[0-9a-fA-F]{3}$|^#?[0-9a-fA-F]{6}$/.test(value.trim());
}

function normalizeHex(hex: string): string {
  let h = hex.trim().replace(/^#/, "");
  if (h.length === 3)
    h = h
      .split("")
      .map((c) => c + c)
      .join("");
  return h.toLowerCase();
}

export function hexToRgb(hex: string): [number, number, number] {
  const h = normalizeHex(hex);
  return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
}

export function rgbToHex(r: number, g: number, b: number): string {
  const c = (n: number) => clamp(Math.round(n), 0, 255).toString(16).padStart(2, "0");
  return `#${c(r)}${c(g)}${c(b)}`;
}

export function hexToHsl(hex: string): [number, number, number] {
  const [r, g, b] = hexToRgb(hex).map((v) => v / 255);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  if (max === min) return [0, 0, l * 100];
  const d = max - min;
  const s = d / (1 - Math.abs(2 * l - 1));
  let h: number;
  if (max === r) h = ((g - b) / d) % 6;
  else if (max === g) h = (b - r) / d + 2;
  else h = (r - g) / d + 4;
  h *= 60;
  if (h < 0) h += 360;
  return [h, s * 100, l * 100];
}

export function hslToHex(h: number, s: number, l: number): string {
  const hue = ((h % 360) + 360) % 360;
  const sat = clamp(s, 0, 100) / 100;
  const light = clamp(l, 0, 100) / 100;
  const c = (1 - Math.abs(2 * light - 1)) * sat;
  const x = c * (1 - Math.abs(((hue / 60) % 2) - 1));
  const m = light - c / 2;
  let r: number, g: number, b: number;
  if (hue < 60) [r, g, b] = [c, x, 0];
  else if (hue < 120) [r, g, b] = [x, c, 0];
  else if (hue < 180) [r, g, b] = [0, c, x];
  else if (hue < 240) [r, g, b] = [0, x, c];
  else if (hue < 300) [r, g, b] = [x, 0, c];
  else [r, g, b] = [c, 0, x];
  return rgbToHex((r + m) * 255, (g + m) * 255, (b + m) * 255);
}

function mix(hexA: string, hexB: string, amount: number): string {
  const [r1, g1, b1] = hexToRgb(hexA);
  const [r2, g2, b2] = hexToRgb(hexB);
  return rgbToHex(r1 * amount + r2 * (1 - amount), g1 * amount + g2 * (1 - amount), b1 * amount + b2 * (1 - amount));
}

// ---------- Contraste WCAG ----------

function relativeLuminance(hex: string): number {
  const [r, g, b] = hexToRgb(hex).map((v) => {
    const c = v / 255;
    return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrastRatio(a: string, b: string): number {
  const la = relativeLuminance(a);
  const lb = relativeLuminance(b);
  const [hi, lo] = la > lb ? [la, lb] : [lb, la];
  return (hi + 0.05) / (lo + 0.05);
}

/** Blanco o un texto casi negro, el que dé más contraste contra `bg`. Nada de
 * umbral de luminosidad "a ojo": con uno fijo (se probó 0.45) la mitad de los
 * `primary` de modo oscuro de los propios temas incluidos (base, moca,
 * medianoche, lavanda) quedaban mal clasificados — elegían blanco con ~2.5:1
 * de contraste en vez del oscuro que daba 6-8:1. Comparar los dos ratios es
 * siempre correcto, sin umbral que calibrar. */
function pickReadableText(bg: string): string {
  const DARK_TEXT = "#101114";
  return contrastRatio("#ffffff", bg) >= contrastRatio(DARK_TEXT, bg) ? "#ffffff" : DARK_TEXT;
}

export interface ContrastCheck {
  pair: string;
  ratio: number;
  min: number;
  ok: boolean;
}

/** Mismos pares que se calibran a mano en los temas incluidos (ver comentarios en base.css). */
export function contrastChecks(colors: EssentialColors): ContrastCheck[] {
  const pairs: Array<[string, string, string, number]> = [
    ["Texto sobre fondo", colors.fg, colors.bg, 4.5],
    ["Texto apagado sobre fondo", colors.muted, colors.bg, 4.5],
    ["Enlaces sobre fondo", colors.link, colors.bg, 4.5],
    // El título de la nota usa --primary como color de texto (ver editor.css).
    ["Título (acento) sobre fondo", colors.primary, colors.bg, 4.5],
  ];
  return pairs.map(([pair, a, b, min]) => {
    const ratio = Math.round(contrastRatio(a, b) * 100) / 100;
    return { pair, ratio, min, ok: ratio >= min };
  });
}

// ---------- Borrador de modo oscuro ----------

const NEUTRAL_DARK_RANGE: Record<"bg" | "surface" | "surfaceAlt" | "border" | "muted" | "fg", { min: number; max: number; desaturate: number }> = {
  bg: { min: 6, max: 16, desaturate: 0.6 },
  surface: { min: 12, max: 24, desaturate: 0.65 },
  surfaceAlt: { min: 16, max: 30, desaturate: 0.7 },
  border: { min: 22, max: 38, desaturate: 0.75 },
  muted: { min: 55, max: 75, desaturate: 1 },
  fg: { min: 86, max: 96, desaturate: 1 },
};

function invertNeutral(hex: string, key: keyof typeof NEUTRAL_DARK_RANGE): string {
  const [h, s, l] = hexToHsl(hex);
  const range = NEUTRAL_DARK_RANGE[key];
  return hslToHex(h, s * range.desaturate, clamp(100 - l, range.min, range.max));
}

function invertAccent(hex: string): string {
  const [h, s, l] = hexToHsl(hex);
  return hslToHex(h, clamp(s * 0.9, 20, 95), clamp(l + 20, 55, 82));
}

/** Borrador de partida para el modo oscuro: invierte la luminosidad de cada
 * color del modo claro. Pensado para editarse antes de guardar, no para ser
 * el resultado final. */
export function invertForDark(light: EssentialColors): EssentialColors {
  return {
    bg: invertNeutral(light.bg, "bg"),
    surface: invertNeutral(light.surface, "surface"),
    surfaceAlt: invertNeutral(light.surfaceAlt, "surfaceAlt"),
    border: invertNeutral(light.border, "border"),
    muted: invertNeutral(light.muted, "muted"),
    fg: invertNeutral(light.fg, "fg"),
    primary: invertAccent(light.primary),
    link: invertAccent(light.link),
    focus: invertAccent(light.focus),
    success: invertAccent(light.success),
    warning: invertAccent(light.warning),
    danger: invertAccent(light.danger),
  };
}

// ---------- Variables derivadas (no editables en el formulario) ----------

export interface DependentColors {
  onPrimary: string;
  primarySoft: string;
  mark: string;
  codeFg: string;
  folder: string;
  shadow: string;
  hlKeyword: string;
  hlString: string;
  hlNumber: string;
  hlComment: string;
  hlTitle: string;
  hlAttr: string;
}

/** Deriva el resto de variables a partir de los ~12 tokens esenciales. Las
 * reglas (hue rotado desde `primary` para code-fg/folder/hl-*, sombra tintada
 * con `fg` en claro y negra en oscuro, mark siempre amarillo) salen de mirar
 * los 6 temas incluidos: ver `scripts/colores.py` para la verificación real. */
export function deriveDependents(colors: EssentialColors, mode: "light" | "dark"): DependentColors {
  const dark = mode === "dark";
  const [hue] = hexToHsl(colors.primary);
  const secondary = (deg: number, s: number, l: number) => hslToHex(hue + deg, s, l);
  const l = dark ? { low: 72, mid: 76, high: 80 } : { low: 34, mid: 38, high: 42 };
  return {
    onPrimary: pickReadableText(colors.primary),
    primarySoft: mix(colors.primary, colors.bg, dark ? 0.22 : 0.14),
    mark: hslToHex(46, dark ? 55 : 85, dark ? 26 : 74),
    codeFg: secondary(150, 50, l.mid),
    folder: secondary(165, 58, l.high),
    shadow: dark ? "0 10px 30px rgb(0 0 0 / 0.5)" : `0 10px 30px rgb(${hexToRgb(colors.fg).join(" ")} / 0.16)`,
    hlKeyword: secondary(280, 45, l.low),
    hlString: secondary(120, 40, l.mid),
    hlNumber: secondary(150, 50, l.high),
    hlComment: colors.muted,
    hlTitle: secondary(205, 50, l.mid),
    hlAttr: secondary(165, 55, l.high),
  };
}

// ---------- Ensamblado del archivo CSS ----------

function block(selector: string, colors: EssentialColors, deps: DependentColors, scheme: "light" | "dark"): string {
  const lines = [
    `  --bg: ${colors.bg};`,
    `  --surface: ${colors.surface};`,
    `  --surface-2: ${colors.surfaceAlt};`,
    `  --border: ${colors.border};`,
    `  --fg: ${colors.fg};`,
    `  --muted: ${colors.muted};`,
    `  --primary: ${colors.primary};`,
    `  --on-primary: ${deps.onPrimary};`,
    `  --primary-soft: ${deps.primarySoft};`,
    `  --link: ${colors.link};`,
    `  --success: ${colors.success};`,
    `  --warning: ${colors.warning};`,
    `  --danger: ${colors.danger};`,
    `  --mark: ${deps.mark};`,
    `  --code-fg: ${deps.codeFg};`,
    `  --focus: ${colors.focus};`,
    `  --folder: ${deps.folder};`,
    `  --shadow: ${deps.shadow};`,
    `  --hl-keyword: ${deps.hlKeyword};`,
    `  --hl-string: ${deps.hlString};`,
    `  --hl-number: ${deps.hlNumber};`,
    `  --hl-comment: ${deps.hlComment};`,
    `  --hl-title: ${deps.hlTitle};`,
    `  --hl-attr: ${deps.hlAttr};`,
    `  color-scheme: ${scheme};`,
  ];
  return `${selector} {\n${lines.join("\n")}\n}`;
}

export function buildThemeCss(displayName: string, light: EssentialColors, dark: EssentialColors): string {
  const header = `/* Tema «${displayName}», creado desde Apariencia → Crear tema. Puedes editar cualquier valor a mano: los cambios se ven al volver a la app. */\n`;
  const lightBlock = block(":root", light, deriveDependents(light, "light"), "light");
  const darkBlock = block(':root[data-color-scheme="dark"]', dark, deriveDependents(dark, "dark"), "dark");
  return `${header}${lightBlock}\n${darkBlock}\n`;
}
