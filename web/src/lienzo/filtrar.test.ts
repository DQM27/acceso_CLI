import { describe, expect, it } from "vitest";
import type { Origen } from "./catalogo";
import type { Fila } from "./calculo";
import { aplicarFiltros } from "./filtrar";
import type { Filtro } from "./esquema";

const origen: Origen = {
  columnas: [
    { nombre: "empresa", tipo: "texto" },
    { nombre: "dia", tipo: "fecha" },
    { nombre: "n", tipo: "numero" },
    { nombre: "activo", tipo: "booleano" },
  ],
};

const filas: Fila[] = [
  { empresa: "ALDAMA", dia: "2026-10-08", n: 3, activo: true },
  { empresa: "Pérez y Hnos", dia: "2026-10-20T14:30:00+00:00", n: 10, activo: false },
  { empresa: null, dia: null, n: 0, activo: true },
];

const con = (...filtros: Filtro[]) => aplicarFiltros(origen, filas, filtros);
const empresas = (r: Fila[]) => r.map((f) => f.empresa);

describe("aplicarFiltros", () => {
  it("sin filtros devuelve todo, y ignora los de campos que la consulta no tiene", () => {
    expect(con()).toHaveLength(3);
    expect(con({ campo: "otra", op: "igual", valor: "x" })).toHaveLength(3);
  });

  it("texto: igual y en sin distinguir mayúsculas ni tildes; contiene", () => {
    expect(empresas(con({ campo: "empresa", op: "igual", valor: "aldama" }))).toEqual(["ALDAMA"]);
    expect(empresas(con({ campo: "empresa", op: "en", valor: ["perez y hnos", "x"] }))).toEqual(["Pérez y Hnos"]);
    expect(empresas(con({ campo: "empresa", op: "contiene", valor: "PEREZ" }))).toEqual(["Pérez y Hnos"]);
  });

  it("no_en deja pasar los vacíos: no están en la lista", () => {
    expect(empresas(con({ campo: "empresa", op: "no_en", valor: ["ALDAMA"] }))).toEqual(["Pérez y Hnos", null]);
  });

  it("un vacío no cumple igual, en, contiene ni entre", () => {
    expect(con({ campo: "empresa", op: "igual", valor: "" })).toHaveLength(0);
    expect(con({ campo: "dia", op: "entre", valor: ["2000-01-01", "2100-01-01"] })).toHaveLength(2);
  });

  it("fecha: compara el día aunque venga con hora", () => {
    expect(con({ campo: "dia", op: "igual", valor: "2026-10-20" })).toHaveLength(1);
    expect(con({ campo: "dia", op: "entre", valor: ["2026-10-08", "2026-10-20"] })).toHaveLength(2);
    expect(con({ campo: "dia", op: "entre", valor: ["2026-10-09", "2026-10-19"] })).toHaveLength(0);
  });

  it("número y booleano", () => {
    expect(con({ campo: "n", op: "entre", valor: [1, 5] })).toHaveLength(1);
    expect(con({ campo: "n", op: "igual", valor: 0 })).toHaveLength(1);
    expect(con({ campo: "activo", op: "igual", valor: true })).toHaveLength(2);
  });

  it("varios filtros se combinan con Y", () => {
    const r = con({ campo: "activo", op: "igual", valor: true }, { campo: "n", op: "entre", valor: [1, 5] });
    expect(empresas(r)).toEqual(["ALDAMA"]);
  });
});
