#!/usr/bin/env python3
"""Genera los temas incluidos (src/themes/*.css) a partir de especificaciones OKLCH.

Uso, desde la raíz del proyecto:
  python3 scripts/generar-temas.py

Cada tema se describe abajo con tuplas (L, C, H). Los neutros (bg, surface,
surface-2, border, fg) se usan tal cual; el resto se aclara u oscurece solo lo
justo hasta alcanzar el contraste mínimo (4.6:1 para texto, 3:1 para foco y
carpetas; 7.2:1 en los de alto contraste). Si algo no cuadra, falla con un
AssertionError antes de escribir el archivo.

SOBRESCRIBE los .css que genera: los cambios se hacen aquí, no en el CSS.
No genera alto-contraste.css (está escrito a mano) ni index.ts.
Al terminar ejecuta la auditoría de scripts/colores.py.
"""
import importlib.util
import sys
from pathlib import Path

sys.dont_write_bytecode = True  # no dejar scripts/__pycache__ al importar colores.py
ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("colores", ROOT / "scripts/colores.py")
c = importlib.util.module_from_spec(spec)
spec.loader.exec_module(c)
X=c.oklch_a_hex; fit=c.ajustar; cr=c.contraste

def build(n, dark, txt=4.6, ui=3.0):
    """n: dict con neutros (L,C,H) y acentos (L,C,H)."""
    v={}
    for k in ("bg","surface","surface-2","border","fg","muted"): v[k]=X(*n[k])
    B=[v["bg"],v["surface"]]
    v["muted"]=fit(*n["muted"],B+[v["surface-2"]],txt)
    if dark:
        v["primary"]=fit(*n["primary"],B,max(txt,4.6)); v["on-primary"]=v["bg"]
    else:
        v["primary"]=fit(*n["primary"],B+["#ffffff"],max(txt,4.6)); v["on-primary"]="#ffffff"
    v["primary-soft"]=X(*n["primary-soft"])
    for k in ("link","success","warning","danger","code-fg"): v[k]=fit(*n[k],B,txt)
    if dark: v["danger"]=fit(*c.oklch(v["danger"]),[v["bg"]],txt)  # bg como texto sobre danger
    v["mark"]=X(*n["mark"])
    v["focus"]=fit(*n["focus"],B,ui)
    v["folder"]=fit(*n["folder"],B,ui)
    for k in ("keyword","string","number","comment","title","attr"): v["hl-"+k]=fit(*n["hl-"+k],B,txt)
    # comprobaciones
    assert cr(v["fg"],v["bg"])>=7, ("fg",cr(v["fg"],v["bg"]))
    assert cr(v["fg"],v["mark"])>=4.5, ("mark",cr(v["fg"],v["mark"]))
    assert cr(v["fg"],v["primary-soft"])>=7, ("soft",cr(v["fg"],v["primary-soft"]))
    assert cr(v["on-primary"],v["primary"])>=4.5
    assert cr(v["bg"],v["danger"])>=4.5
    return v

ORDER=["bg","surface","surface-2","border","fg","muted","primary","on-primary","primary-soft","link","success","warning","danger","mark","code-fg","focus","folder"]
def block(sel,v,shadow,scheme):
    lines=[f"{sel} {{"]+[f"  --{k}: {v[k]};" for k in ORDER]
    lines.append(f"  --shadow: {shadow};")
    lines.append("  "+" ".join(f"--hl-{k}: {v['hl-'+k]};" for k in ("keyword","string","number","comment","title","attr")))
    lines.append(f"  color-scheme: {scheme};"); lines.append("}")
    return "\n".join(lines)

def write(name, header, light, dark, shL, shD, extra="", txt=4.6):
    vl=build(light,False,txt); vd=build(dark,True,txt)
    css=header.strip()+"\n"+block(":root",vl,shL,"light")+"\n"+block(':root[data-color-scheme="dark"]',vd,shD,"dark")+"\n"+extra
    (ROOT/f"src/themes/{name}.css").write_text(css)
    print("ok",name)

# ---------------- Medianoche: navy, ice blue, electric blue, chartreuse ----------------
write("medianoche","""/* Medianoche — azul marino, azul hielo, azul eléctrico y un toque de chartreuse.
   Reparto 60-30-10: marino/hielo de base, azul eléctrico para las acciones y
   chartreuse solo en detalles (foco, código, carpetas). */""",
 light={"bg":(0.985,0.006,235),"surface":(0.962,0.012,240),"surface-2":(0.93,0.018,245),"border":(0.875,0.025,250),
  "fg":(0.24,0.06,262),"muted":(0.47,0.045,258),"primary":(0.52,0.23,264),"primary-soft":(0.925,0.04,262),
  "link":(0.48,0.22,264),"success":(0.52,0.12,160),"warning":(0.55,0.12,70),"danger":(0.53,0.18,22),
  "mark":(0.93,0.16,122),"code-fg":(0.50,0.14,128),"focus":(0.60,0.24,264),"folder":(0.62,0.17,125),
  "hl-keyword":(0.50,0.20,264),"hl-string":(0.50,0.14,128),"hl-number":(0.52,0.12,200),"hl-comment":(0.52,0.03,255),"hl-title":(0.50,0.14,290),"hl-attr":(0.52,0.10,230)},
 dark={"bg":(0.18,0.045,262),"surface":(0.215,0.05,262),"surface-2":(0.26,0.055,262),"border":(0.32,0.055,262),
  "fg":(0.95,0.02,230),"muted":(0.76,0.035,240),"primary":(0.68,0.19,262),"primary-soft":(0.30,0.09,262),
  "link":(0.78,0.12,245),"success":(0.76,0.12,160),"warning":(0.80,0.12,75),"danger":(0.72,0.15,22),
  "mark":(0.42,0.10,125),"code-fg":(0.90,0.19,124),"focus":(0.90,0.20,124),"folder":(0.88,0.18,124),
  "hl-keyword":(0.74,0.15,262),"hl-string":(0.88,0.18,124),"hl-number":(0.82,0.10,200),"hl-comment":(0.70,0.035,250),"hl-title":(0.82,0.10,230),"hl-attr":(0.78,0.10,290)},
 shL="0 10px 30px rgb(15 30 70 / 0.16)", shD="0 10px 30px rgb(0 0 0 / 0.5)")

# ---------------- Moca: mocha mousse, crema, ciruela, oro ----------------
write("moca","""/* Moca — neutros cálidos «mocha mousse» y crema, ciruela para las acciones y
   oro viejo como acento. Elegante y tranquilo para leer mucho rato. */""",
 light={"bg":(0.975,0.009,75),"surface":(0.952,0.014,70),"surface-2":(0.92,0.02,65),"border":(0.86,0.028,60),
  "fg":(0.27,0.03,45),"muted":(0.48,0.035,50),"primary":(0.44,0.12,350),"primary-soft":(0.93,0.025,350),
  "link":(0.42,0.12,350),"success":(0.50,0.10,145),"warning":(0.52,0.11,70),"danger":(0.50,0.17,25),
  "mark":(0.90,0.10,88),"code-fg":(0.48,0.07,45),"focus":(0.60,0.12,85),"folder":(0.62,0.11,80),
  "hl-keyword":(0.46,0.12,350),"hl-string":(0.50,0.09,140),"hl-number":(0.52,0.10,70),"hl-comment":(0.52,0.025,55),"hl-title":(0.48,0.07,45),"hl-attr":(0.50,0.08,230)},
 dark={"bg":(0.20,0.014,50),"surface":(0.235,0.017,50),"surface-2":(0.28,0.02,50),"border":(0.34,0.024,50),
  "fg":(0.93,0.018,75),"muted":(0.74,0.03,65),"primary":(0.74,0.10,350),"primary-soft":(0.32,0.05,350),
  "link":(0.80,0.09,350),"success":(0.76,0.10,145),"warning":(0.80,0.11,75),"danger":(0.72,0.14,25),
  "mark":(0.42,0.07,85),"code-fg":(0.82,0.10,85),"focus":(0.82,0.12,85),"folder":(0.80,0.11,82),
  "hl-keyword":(0.76,0.10,350),"hl-string":(0.78,0.09,140),"hl-number":(0.80,0.10,70),"hl-comment":(0.68,0.025,60),"hl-title":(0.82,0.10,85),"hl-attr":(0.78,0.07,230)},
 shL="0 10px 30px rgb(60 40 25 / 0.16)", shD="0 10px 30px rgb(0 0 0 / 0.5)")

# ---------------- Salvia: salvia, hueso, verde bosque, terracota ----------------
write("salvia","""/* Salvia — fondo hueso con tinte salvia, verde bosque para las acciones y
   terracota como acento. Natural y sereno. */""",
 light={"bg":(0.978,0.008,110),"surface":(0.955,0.014,135),"surface-2":(0.925,0.02,140),"border":(0.87,0.028,145),
  "fg":(0.26,0.03,165),"muted":(0.47,0.03,160),"primary":(0.45,0.08,165),"primary-soft":(0.92,0.03,160),
  "link":(0.44,0.08,175),"success":(0.52,0.12,135),"warning":(0.54,0.11,72),"danger":(0.52,0.17,22),
  "mark":(0.91,0.08,95),"code-fg":(0.52,0.12,45),"focus":(0.58,0.10,165),"folder":(0.64,0.12,48),
  "hl-keyword":(0.48,0.08,170),"hl-string":(0.52,0.11,130),"hl-number":(0.54,0.12,45),"hl-comment":(0.54,0.02,150),"hl-title":(0.50,0.07,230),"hl-attr":(0.54,0.10,75)},
 dark={"bg":(0.20,0.014,165),"surface":(0.235,0.017,165),"surface-2":(0.28,0.02,165),"border":(0.34,0.024,160),
  "fg":(0.94,0.012,110),"muted":(0.75,0.025,150),"primary":(0.78,0.08,160),"primary-soft":(0.32,0.045,165),
  "link":(0.82,0.07,175),"success":(0.80,0.11,135),"warning":(0.82,0.10,80),"danger":(0.72,0.14,22),
  "mark":(0.42,0.06,95),"code-fg":(0.76,0.11,45),"focus":(0.80,0.10,160),"folder":(0.76,0.11,48),
  "hl-keyword":(0.80,0.08,170),"hl-string":(0.82,0.10,130),"hl-number":(0.78,0.11,45),"hl-comment":(0.68,0.02,150),"hl-title":(0.80,0.07,230),"hl-attr":(0.82,0.09,80)},
 shL="0 10px 30px rgb(30 50 40 / 0.15)", shD="0 10px 30px rgb(0 0 0 / 0.5)")

# ---------------- Lavanda: berenjena, lavanda digital, menta ----------------
write("lavanda","""/* Lavanda — neutros con matiz berenjena, lavanda digital para las acciones y
   menta como contrapunto fresco. */""",
 light={"bg":(0.982,0.007,305),"surface":(0.958,0.014,305),"surface-2":(0.928,0.02,305),"border":(0.875,0.03,305),
  "fg":(0.26,0.05,310),"muted":(0.48,0.04,305),"primary":(0.50,0.16,298),"primary-soft":(0.925,0.035,300),
  "link":(0.47,0.16,298),"success":(0.52,0.11,155),"warning":(0.55,0.11,70),"danger":(0.52,0.18,20),
  "mark":(0.92,0.07,170),"code-fg":(0.50,0.09,175),"focus":(0.58,0.16,298),"folder":(0.66,0.10,172),
  "hl-keyword":(0.50,0.16,298),"hl-string":(0.50,0.09,175),"hl-number":(0.52,0.14,350),"hl-comment":(0.52,0.03,300),"hl-title":(0.50,0.12,255),"hl-attr":(0.54,0.10,70)},
 dark={"bg":(0.19,0.03,305),"surface":(0.225,0.035,305),"surface-2":(0.27,0.04,305),"border":(0.33,0.045,305),
  "fg":(0.95,0.015,305),"muted":(0.76,0.04,305),"primary":(0.76,0.12,298),"primary-soft":(0.32,0.07,300),
  "link":(0.80,0.10,298),"success":(0.80,0.11,155),"warning":(0.82,0.11,75),"danger":(0.72,0.15,20),
  "mark":(0.42,0.07,172),"code-fg":(0.84,0.10,172),"focus":(0.84,0.11,172),"folder":(0.82,0.10,172),
  "hl-keyword":(0.78,0.12,298),"hl-string":(0.84,0.10,172),"hl-number":(0.78,0.12,350),"hl-comment":(0.68,0.035,300),"hl-title":(0.80,0.09,255),"hl-attr":(0.82,0.10,75)},
 shL="0 10px 30px rgb(50 30 70 / 0.15)", shD="0 10px 30px rgb(0 0 0 / 0.5)")

# ---------------- Medianoche AAA (alto contraste) ----------------
write("medianoche-aaa","""/* Medianoche AAA — alto contraste sobre la paleta Medianoche: texto a 7:1 o más,
   bordes marcados y foco chartreuse muy visible. */""",
 light={"bg":(1.0,0,0),"surface":(0.97,0.008,240),"surface-2":(0.93,0.014,245),"border":(0.45,0.06,262),
  "fg":(0.18,0.06,262),"muted":(0.30,0.05,260),"primary":(0.40,0.22,264),"primary-soft":(0.92,0.04,262),
  "link":(0.40,0.22,264),"success":(0.40,0.10,160),"warning":(0.42,0.09,70),"danger":(0.42,0.17,25),
  "mark":(0.93,0.17,120),"code-fg":(0.38,0.10,128),"focus":(0.45,0.25,264),"folder":(0.42,0.10,125),
  "hl-keyword":(0.40,0.20,264),"hl-string":(0.38,0.10,128),"hl-number":(0.40,0.09,200),"hl-comment":(0.36,0.03,255),"hl-title":(0.40,0.14,295),"hl-attr":(0.42,0.09,70)},
 dark={"bg":(0.13,0.04,262),"surface":(0.17,0.045,262),"surface-2":(0.23,0.05,262),"border":(0.75,0.04,240),
  "fg":(1.0,0,0),"muted":(0.90,0.02,235),"primary":(0.80,0.14,245),"primary-soft":(0.30,0.08,262),
  "link":(0.85,0.10,240),"success":(0.88,0.13,160),"warning":(0.90,0.13,80),"danger":(0.80,0.12,22),
  "mark":(0.38,0.09,125),"code-fg":(0.93,0.20,124),"focus":(0.93,0.21,124),"folder":(0.90,0.18,124),
  "hl-keyword":(0.85,0.10,262),"hl-string":(0.93,0.20,124),"hl-number":(0.88,0.09,200),"hl-comment":(0.85,0.02,240),"hl-title":(0.88,0.08,295),"hl-attr":(0.90,0.12,80)},
 shL="0 0 0 2px #0f1a3a", shD="0 0 0 2px #ffffff",
 extra="""/* Foco y enlaces más marcados que en las paletas normales. */
:focus-visible { outline: 3px solid var(--focus) !important; outline-offset: 2px; }
.cm-live-link, .markdown-reading a { text-decoration: underline; text-decoration-thickness: 2px; }
""", txt=7.2)

sys.exit(c.revisar())
