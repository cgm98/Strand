// Aplica modo y tema guardados antes del primer pintado (sin destello del
// tema predeterminado). La fuente de verdad es useAppearance.ts; esto solo
// lee su caché. Si falla, la app arranca con el aspecto predeterminado.
// Archivo aparte (no <script> inline) para no necesitar `'unsafe-inline'` en
// script-src de la CSP (ver tauri.conf.json).
(function () {
  var cache = null;
  try {
    cache = JSON.parse(localStorage.getItem("notas.appearance") || "null");
  } catch {
    // Caché de comodidad: si está corrupta, arranca con el aspecto predeterminado.
  }
  var mode = (cache && cache.colorScheme) || "system";
  if (mode === "system") mode = matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  document.documentElement.dataset.colorScheme = mode;
  if (!cache || !Array.isArray(cache.sheets)) return;
  var host = document.createElement("div");
  host.id = "user-appearance";
  host.hidden = true;
  cache.sheets.forEach(function (sheet) {
    var style = document.createElement("style");
    style.dataset.appearance = sheet.id;
    style.textContent = sheet.css;
    host.appendChild(style);
  });
  document.body.appendChild(host);
})();
