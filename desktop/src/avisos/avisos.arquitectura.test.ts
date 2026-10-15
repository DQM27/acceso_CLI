import { describe, expect, it } from "vitest";

/** Todo el código de `src/` como texto (Vite lo resuelve al armar el test). */
const codigo = import.meta.glob<string>(["../**/*.ts", "../**/*.tsx"], {
  query: "?raw",
  import: "default",
  eager: true,
});

describe("arquitectura de los avisos", () => {
  it("ninguna pantalla ni componente importa `sonner` directo: todo pasa por `avisos/`", () => {
    const infractores = Object.entries(codigo)
      // Sólo `avisos/` puede hablar con el motor; los tests pueden simularlo.
      .filter(([ruta]) => !ruta.startsWith("./") && !/\.test\.tsx?$/.test(ruta))
      .filter(([, texto]) => /from\s+["']sonner["']/.test(texto))
      .map(([ruta]) => ruta);
    expect(infractores).toEqual([]);
  });

  it("el recorrido sí ve los archivos (que el test no pase por estar vacío)", () => {
    expect(Object.keys(codigo).length).toBeGreaterThan(20);
  });
});
