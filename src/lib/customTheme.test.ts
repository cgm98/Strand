import { describe, expect, test } from "bun:test";
import {
  DEFAULT_LIGHT as LIGHT,
  buildThemeCss,
  contrastChecks,
  contrastRatio,
  deriveDependents,
  hexToHsl,
  hexToRgb,
  hslToHex,
  invertForDark,
  isValidHex,
  rgbToHex,
} from "./customTheme";

describe("isValidHex", () => {
  test("acepta 3 y 6 dígitos, con o sin #", () => {
    expect(isValidHex("#fff")).toBe(true);
    expect(isValidHex("fff")).toBe(true);
    expect(isValidHex("#4e4ecc")).toBe(true);
    expect(isValidHex("4e4ecc")).toBe(true);
  });
  test("rechaza formatos inválidos", () => {
    expect(isValidHex("")).toBe(false);
    expect(isValidHex("rgb(0,0,0)")).toBe(false);
    expect(isValidHex("#12345")).toBe(false);
    expect(isValidHex("#gggggg")).toBe(false);
  });
});

describe("hex <-> rgb <-> hsl", () => {
  test("hexToRgb y rgbToHex son inversos", () => {
    expect(hexToRgb("#ff0000")).toEqual([255, 0, 0]);
    expect(rgbToHex(255, 0, 0)).toBe("#ff0000");
  });
  test("hexToHsl y hslToHex redondean al mismo color", () => {
    const [h, s, l] = hexToHsl("#4e4ecc");
    expect(hslToHex(h, s, l).toLowerCase()).toBe("#4e4ecc");
  });
  test("gris puro no tiene matiz (hue 0, sat 0)", () => {
    expect(hexToHsl("#808080")).toEqual([0, 0, hexToHsl("#808080")[2]]);
  });
});

describe("contrastRatio / contrastChecks", () => {
  test("blanco contra negro da el máximo (21:1)", () => {
    expect(contrastRatio("#ffffff", "#000000")).toBeCloseTo(21, 0);
  });
  test("mismo color da 1:1", () => {
    expect(contrastRatio("#4e4ecc", "#4e4ecc")).toBeCloseTo(1, 1);
  });
  test("detecta la paleta base como accesible (fg/muted/link sobre bg)", () => {
    const checks = contrastChecks(LIGHT);
    for (const c of checks) expect(c.ok).toBe(true);
  });
  test("marca un par que no llega a 4.5:1", () => {
    const low = { ...LIGHT, muted: "#d0d2d8" }; // gris claro casi igual al fondo
    const checks = contrastChecks(low);
    const muted = checks.find((c) => c.pair.includes("apagado"));
    expect(muted?.ok).toBe(false);
  });
});

describe("invertForDark", () => {
  const dark = invertForDark(LIGHT);
  test("invierte la luminosidad del fondo y el texto (oscurece/aclara)", () => {
    expect(hexToHsl(dark.bg)[2]).toBeLessThan(hexToHsl(LIGHT.bg)[2]);
    expect(hexToHsl(dark.fg)[2]).toBeGreaterThan(hexToHsl(LIGHT.fg)[2]);
  });
  test("los acentos quedan dentro de un rango razonable (no negros ni blancos puros)", () => {
    for (const key of ["primary", "link", "focus", "success", "warning", "danger"] as const) {
      const l = hexToHsl(dark[key])[2];
      expect(l).toBeGreaterThan(40);
      expect(l).toBeLessThan(95);
    }
  });
});

describe("deriveDependents", () => {
  test("elige on-primary legible (blanco sobre primary oscuro, oscuro sobre primary claro)", () => {
    const onDark = deriveDependents({ ...LIGHT, primary: "#101014" }, "light").onPrimary;
    const onLight = deriveDependents({ ...LIGHT, primary: "#f5f5f5" }, "light").onPrimary;
    expect(onDark).toBe("#ffffff");
    expect(onLight).toBe("#101114");
  });
  test("acierta en primarios de luminosidad media (antes se elegía por un umbral fijo de 0.45 y fallaba aquí)", () => {
    // #db91b4 y #5d94ff son los `--primary` de modo oscuro reales de Moca y
    // Medianoche: ~0.3-0.4 de luminosidad relativa, por debajo del viejo
    // umbral de 0.45, así que el código anterior elegía blanco (~2.4:1) en
    // vez del oscuro que de verdad contrasta mejor (~6.3-7.8:1).
    expect(deriveDependents({ ...LIGHT, primary: "#db91b4" }, "dark").onPrimary).toBe("#101114");
    expect(deriveDependents({ ...LIGHT, primary: "#5d94ff" }, "dark").onPrimary).toBe("#101114");
  });
  test("la sombra de modo oscuro es siempre negra; la de claro usa el tinte de fg", () => {
    const deps = deriveDependents(LIGHT, "light");
    expect(deps.shadow).toContain(hexToRgb(LIGHT.fg).join(" "));
    expect(deriveDependents(invertForDark(LIGHT), "dark").shadow).toBe("0 10px 30px rgb(0 0 0 / 0.5)");
  });
  test("hl-comment reutiliza el color de texto apagado", () => {
    expect(deriveDependents(LIGHT, "light").hlComment).toBe(LIGHT.muted);
  });
});

describe("buildThemeCss", () => {
  const css = buildThemeCss("Mi tema", LIGHT, invertForDark(LIGHT));
  test("genera los dos bloques de :root con todas las variables que usan los temas incluidos", () => {
    expect(css).toContain(":root {");
    expect(css).toContain(':root[data-color-scheme="dark"] {');
    for (const name of ["bg", "surface", "surface-2", "border", "fg", "muted", "primary", "on-primary", "primary-soft", "link", "focus", "shadow", "folder"])
      expect(css).toContain(`--${name}:`);
  });
  test("incluye el nombre del tema en el comentario de cabecera", () => {
    expect(css).toContain("Mi tema");
  });
});
