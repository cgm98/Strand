import { describe, expect, test } from "bun:test";
import { sanitizeHtml, sanitizeSvg } from "./sanitize";

describe("sanitizeHtml", () => {
  test("quita <script> y manejadores de eventos inline", () => {
    const dirty = '<p onclick="evil()">hola</p><script>evil()</script>';
    const clean = sanitizeHtml(dirty);
    expect(clean).not.toContain("<script");
    expect(clean).not.toContain("onclick");
    expect(clean).toContain("hola");
  });

  test("deja pasar enlaces http/https normales", () => {
    const clean = sanitizeHtml('<a href="https://example.com">enlace</a>');
    expect(clean).toContain('href="https://example.com"');
  });

  test("deja pasar el esquema attachment:// (imágenes propias)", () => {
    const clean = sanitizeHtml('<img src="attachment://foo.png" alt="">');
    expect(clean).toContain('src="attachment://foo.png"');
  });

  test("bloquea el esquema javascript:", () => {
    const clean = sanitizeHtml('<a href="javascript:evil()">enlace</a>');
    expect(clean).not.toContain("javascript:");
  });
});

describe("sanitizeSvg", () => {
  test("deja pasar un <svg> simple", () => {
    const clean = sanitizeSvg('<svg viewBox="0 0 10 10"><circle cx="5" cy="5" r="4"/></svg>');
    expect(clean).toContain("<svg");
    expect(clean).toContain("<circle");
  });

  test("quita <script> embebido dentro de un SVG", () => {
    const clean = sanitizeSvg("<svg><script>evil()</script><rect/></svg>");
    expect(clean).not.toContain("<script");
  });
});
