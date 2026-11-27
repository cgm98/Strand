// Temas incluidos en la app. Van dentro del ejecutable (Vite `?raw`), no en la
// carpeta del usuario: se actualizan con la app y no se pueden romper sin querer.
// Para personalizar uno, el diálogo de Apariencia lo duplica en la carpeta.
//
// Los colores se revisan con `python3 scripts/colores.py` (contraste WCAG y armonía).
import medianoche from "./medianoche.css?raw";
import moca from "./moca.css?raw";
import salvia from "./salvia.css?raw";
import lavanda from "./lavanda.css?raw";
import altoContraste from "./alto-contraste.css?raw";
import medianocheAaa from "./medianoche-aaa.css?raw";

/** «paleta»: temas de color normales (AA). «alto-contraste»: texto a 7:1 o más (AAA). */
export type ThemeCategory = "paleta" | "alto-contraste";

export interface BuiltinTheme {
  id: string;
  name: string;
  description: string;
  category: ThemeCategory;
  css: string;
}

export const builtinThemes: BuiltinTheme[] = [
  {
    id: "medianoche",
    name: "Medianoche",
    description: "Azul marino, azul hielo y azul eléctrico, con detalles en chartreuse.",
    category: "paleta",
    css: medianoche,
  },
  { id: "moca", name: "Moca", description: "Crema y moca cálidos, con ciruela y oro viejo.", category: "paleta", css: moca },
  { id: "salvia", name: "Salvia", description: "Hueso y salvia, con verde bosque y terracota.", category: "paleta", css: salvia },
  { id: "lavanda", name: "Lavanda", description: "Berenjena y lavanda digital, con menta.", category: "paleta", css: lavanda },
  { id: "alto-contraste", name: "Alto contraste", description: "Blanco y negro puros, con foco muy visible.", category: "alto-contraste", css: altoContraste },
  { id: "medianoche-aaa", name: "Medianoche AAA", description: "La paleta Medianoche con texto a 7:1 o más.", category: "alto-contraste", css: medianocheAaa },
];

export const themeCategories: Array<{ id: ThemeCategory; label: string }> = [
  { id: "paleta", label: "Paletas" },
  { id: "alto-contraste", label: "Alto contraste" },
];

// Antes solo bg/surface/fg/primary/focus: con eso no se veía la "escala de
// grises" del tema (superficie elevada, borde, texto apagado) hasta elegirlo
// y mirar la app entera. Con surface-2/border/muted en la muestra, se compara
// de un vistazo también esa parte, no solo el acento.
/** Colores de muestra (escala neutra + acento) sacados del propio CSS. */
export function swatches(css: string, scheme: "light" | "dark"): string[] {
  const selector = scheme === "dark" ? /:root\[data-color-scheme="dark"\]\s*\{([^}]*)\}/ : /:root\s*\{([^}]*)\}/;
  const block = selector.exec(css)?.[1] ?? "";
  const value = (name: string) => new RegExp(`--${name}:\\s*([^;]+);`).exec(block)?.[1].trim();
  return ["bg", "surface", "surface-2", "border", "muted", "fg", "primary", "focus"].map(value).filter((v): v is string => !!v);
}
