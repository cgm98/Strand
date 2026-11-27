// Portado de cogni-vault: frontmatter YAML mínimo (clave: valor por línea),
// suficiente para sacar el `title` al importar archivos .md sueltos.
export interface ParsedFrontmatter {
  data: Record<string, string>;
  body: string;
}

export function parseFrontmatter(markdown: string): ParsedFrontmatter {
  if (!markdown.startsWith("---")) return { data: {}, body: markdown };
  const end = markdown.indexOf("\n---", 3);
  if (end === -1) return { data: {}, body: markdown };
  const data: Record<string, string> = {};
  for (const line of markdown.slice(3, end).trim().split("\n")) {
    const colon = line.indexOf(":");
    if (colon !== -1)
      data[line.slice(0, colon).trim()] = line
        .slice(colon + 1)
        .trim()
        .replace(/^["']|["']$/g, "");
  }
  return { data, body: markdown.slice(end + 4).trimStart() };
}
