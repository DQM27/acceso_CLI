import { describe, expect, it } from "vitest";
import { definicionesVisibles, generarCsv, generarHtmlTabla } from "./exportacion";
import type { DefinicionColumnaExport } from "./exportacion";

interface Fila {
  a: string;
  b: string;
}
const columnas: DefinicionColumnaExport<Fila>[] = [
  { colId: "a", etiqueta: "Columna A", valor: (f) => f.a },
  { colId: "b", etiqueta: "Columna B", izquierda: true, valor: (f) => f.b },
];

describe("exportación común", () => {
  it("exporta las columnas visibles en el orden de la grilla, o todas sin grilla", () => {
    expect(definicionesVisibles(columnas, ["b", "a"]).map((c) => c.colId)).toEqual(["b", "a"]);
    expect(definicionesVisibles(columnas, ["b", "desconocida"]).map((c) => c.colId)).toEqual(["b"]);
    expect(definicionesVisibles(columnas, undefined).map((c) => c.colId)).toEqual(["a", "b"]);
  });

  it("el PDF lleva el título pedido y escapa el contenido", () => {
    const html = generarHtmlTabla([{ a: "<b>", b: "x" }], columnas, {
      titulo: "Bitácora de sesiones",
      generadoPor: "Admin",
      filtro: "Filtro: todo",
    });
    expect(html).toContain("<h1>Bitácora de sesiones</h1>");
    expect(html).toContain("<title>Bitácora de sesiones</title>");
    expect(html).toContain("&lt;b&gt;");
  });

  it("el CSV usa ; y comillas cuando hace falta", () => {
    expect(generarCsv([{ a: "1;2", b: "ok" }], columnas)).toBe('﻿Columna A;Columna B\r\n"1;2";ok\r\n');
  });
});
