#!/usr/bin/env python3
"""Auditoría de color de los temas (contraste WCAG y armonía en OKLCH).

Uso, desde la raíz del proyecto:
  python3 scripts/colores.py                 revisa todos los temas; sale con 1 si algo falla
  python3 scripts/colores.py tabla           muestra L/C/H y contraste de cada token
  python3 scripts/colores.py ajustar L C H FONDO [FONDO…] [--min 4.5]
                                             busca el hex más cercano a oklch(L C H) que
                                             alcance el contraste mínimo sobre los fondos

Solo usa la biblioteca estándar. Lee src/styles/base.css (valores por defecto y de
respaldo), src/themes/*.css y los --hl-* por defecto de src/components/editor/document.css.
"""
import math
import re
import sys
from pathlib import Path

RAIZ = Path(__file__).resolve().parent.parent
BASE = RAIZ / "src/styles/base.css"
TEMAS = sorted((RAIZ / "src/themes").glob("*.css"))
DOCUMENTO = RAIZ / "src/components/editor/document.css"

SELECTORES = {
    "claro": r":root\s*\{([^}]*)\}",
    "oscuro": r':root\[data-color-scheme="dark"\]\s*\{([^}]*)\}',
}

# Temas de la categoría «alto-contraste» (src/themes/index.ts): texto a 7:1 (AAA).
ALTO_CONTRASTE = {"alto-contraste", "medianoche-aaa"}

# Tokens que se usan como color de texto: deben llegar a 4.5:1 sobre --bg y --surface.
# `primary` entró aquí al pasar a ser el color del título de la nota (antes
# solo se usaba como fondo de botón/acento de UI, nunca como texto de cuerpo).
TEXTO = ["muted", "link", "primary", "success", "warning", "danger", "code-fg",
         "hl-keyword", "hl-string", "hl-number", "hl-comment", "hl-title", "hl-attr"]

# (primer plano, fondo, mínimo)
PARES = [
    ("muted", "surface-2", 4.5),
    ("on-primary", "primary", 4.5),
    ("fg", "primary-soft", 7),
    ("fg", "mark", 4.5),
    ("bg", "danger", 4.5),     # botón de peligro: texto en --bg sobre --danger
    ("focus", "bg", 3),        # componente de interfaz
    ("folder", "surface", 3),
    ("folder", "bg", 3),
]


# ---------- Conversión de color ----------

def _hex(h):
    h = h.lstrip("#")
    if len(h) == 3:
        h = "".join(c * 2 for c in h)
    return [int(h[i:i + 2], 16) for i in (0, 2, 4)]


def _lineal(c):
    c /= 255
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def luminancia(h):
    r, g, b = map(_lineal, _hex(h))
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def contraste(a, b):
    la, lb = sorted([luminancia(a), luminancia(b)], reverse=True)
    return (la + 0.05) / (lb + 0.05)


def oklch(h):
    r, g, b = map(_lineal, _hex(h))
    l = (0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b) ** (1 / 3)
    m = (0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b) ** (1 / 3)
    s = (0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b) ** (1 / 3)
    L = 0.2104542553 * l + 0.7936177850 * m - 0.0040720468 * s
    A = 1.9779984951 * l - 2.4285922050 * m + 0.4505937099 * s
    B = 0.0259040371 * l + 0.7827717662 * m - 0.8086757660 * s
    return L, math.hypot(A, B), math.degrees(math.atan2(B, A)) % 360


def oklch_a_hex(L, C, H):
    """Convierte a hex reduciendo el croma hasta que el color quepa en sRGB."""
    while True:
        A, B = C * math.cos(math.radians(H)), C * math.sin(math.radians(H))
        l = (L + 0.3963377774 * A + 0.2158037573 * B) ** 3
        m = (L - 0.1055613458 * A - 0.0638541728 * B) ** 3
        s = (L - 0.0894841775 * A - 1.2914855480 * B) ** 3
        rgb = [4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
               -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
               -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s]
        if all(-1e-4 <= v <= 1 + 1e-4 for v in rgb) or C < 0.002:
            break
        C -= 0.002

    def gamma(v):
        v = min(max(v, 0), 1)
        return 12.92 * v if v <= 0.0031308 else 1.055 * v ** (1 / 2.4) - 0.055

    return "#" + "".join(f"{round(gamma(v) * 255):02x}" for v in rgb)


def ajustar(L, C, H, fondos, minimo=4.5):
    """Aleja L del fondo hasta alcanzar el contraste mínimo sobre todos los fondos."""
    paso = -0.005 if luminancia(fondos[0]) > 0.18 else 0.005
    h = oklch_a_hex(L, C, H)
    for _ in range(200):
        if min(contraste(h, f) for f in fondos) >= minimo:
            break
        L += paso
        h = oklch_a_hex(L, C, H)
    return h


# ---------- Lectura de temas ----------

def bloques(css):
    # `re.search` se quedaba con el primer bloque `:root {...}` del archivo:
    # document.css tiene dos (uno para `--block-max-height`, otro para los
    # `--hl-*` de highlight.js más abajo) y el segundo se perdía siempre. Con
    # `finditer` se juntan los tokens de todos los bloques que haya.
    out = {}
    for esquema, sel in SELECTORES.items():
        tokens: dict[str, str] = {}
        encontrado = False
        for m in re.finditer(sel, css):
            encontrado = True
            tokens.update(re.findall(r"--([\w-]+):\s*(#[0-9a-fA-F]{3,6})\b", m.group(1)))
        if encontrado:
            out[esquema] = tokens
    return out


def temas():
    """Devuelve [(nombre, esquema, tokens)] con los huecos rellenos como en la app."""
    defecto = bloques(BASE.read_text())
    hl = bloques(DOCUMENTO.read_text())
    out = []
    for ruta in [BASE] + TEMAS:
        for esquema, tokens in bloques(ruta.read_text()).items():
            completo = {**hl.get(esquema, {}), **defecto[esquema], **tokens}
            out.append((ruta.stem, esquema, completo))
    return out


def _dif_tono(a, b):
    d = abs(a[2] - b[2])
    return min(d, 360 - d)


# ---------- Comandos ----------

def revisar():
    fallos = 0
    for nombre, esquema, v in temas():
        etiqueta = f"{nombre}/{esquema}"
        texto_min = 7.0 if nombre in ALTO_CONTRASTE else 4.5
        for k in TEXTO:
            for fondo in ("bg", "surface"):
                r = contraste(v[k], v[fondo])
                if r < texto_min:
                    print(f"FALLA  {etiqueta}: --{k} sobre --{fondo} = {r:.2f} (mín. {texto_min})")
                    fallos += 1
        for fg, fondo, minimo in PARES:
            r = contraste(v[fg], v[fondo])
            if r < minimo:
                print(f"FALLA  {etiqueta}: --{fg} sobre --{fondo} = {r:.2f} (mín. {minimo})")
                fallos += 1

        # Avisos de armonía: no hacen fallar, pero conviene mirarlos.
        p, d, w, f = (oklch(v[k]) for k in ("primary", "danger", "warning", "focus"))
        if v["primary"] == v["warning"]:
            print(f"AVISO  {etiqueta}: --primary y --warning son el mismo color")
        if _dif_tono(p, d) < 30 and abs(p[1] - d[1]) < 0.05:
            print(f"AVISO  {etiqueta}: --primary y --danger se parecen (ΔH {_dif_tono(p, d):.0f}°)")
        if _dif_tono(f, d) < 20:
            print(f"AVISO  {etiqueta}: --focus tiene el tono de --danger (parece un error)")
        # --danger queda fuera a propósito: debe destacar aunque ocupe poco.
        if nombre not in ALTO_CONTRASTE:
            for k in ("success", "warning"):
                s = oklch(v[k])
                if s[1] > p[1] + 0.05 and s[0] > p[0] + 0.08:
                    print(f"AVISO  {etiqueta}: --{k} es más vistoso que --primary (jerarquía invertida)")

    print(f"\n{len(temas())} combinaciones revisadas, {fallos} fallos.")
    return 1 if fallos else 0


def tabla():
    for nombre, esquema, v in temas():
        print(f"\n### {nombre} — {esquema}")
        print(f"  {'token':12} {'hex':8} {'L':>5} {'C':>6} {'H':>5} {'/bg':>6} {'/surf':>6}")
        for k in ["primary", "link", "focus", "success", "warning", "danger", "code-fg",
                  "folder", "mark"] + [k for k in TEXTO if k.startswith("hl-")]:
            if k in v:
                L, C, H = oklch(v[k])
                print(f"  {k:12} {v[k]:8} {L:5.2f} {C:6.3f} {H:5.0f} "
                      f"{contraste(v[k], v['bg']):6.2f} {contraste(v[k], v['surface']):6.2f}")
    return 0


def cmd_ajustar(args):
    minimo = 4.5
    if "--min" in args:
        i = args.index("--min")
        minimo = float(args[i + 1])
        args = args[:i] + args[i + 2:]
    if len(args) < 4:
        print(__doc__)
        return 2
    L, C, H = map(float, args[:3])
    fondos = args[3:]
    h = ajustar(L, C, H, fondos, minimo)
    l2, c2, h2 = oklch(h)
    ratios = " ".join(f"{contraste(h, f):.2f}" for f in fondos)
    print(f"{h}  oklch({l2:.3f} {c2:.3f} {h2:.1f})  contraste: {ratios}")
    return 0


if __name__ == "__main__":
    args = sys.argv[1:]
    if not args or args[0] == "revisar":
        sys.exit(revisar())
    if args[0] == "tabla":
        sys.exit(tabla())
    if args[0] == "ajustar":
        sys.exit(cmd_ajustar(args[1:]))
    print(__doc__)
    sys.exit(2)
