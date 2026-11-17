import { describe, expect, it } from "vitest";
import type { GafeteParaConteo, ResultadoConteo } from "../api";
import {
  agruparTablero,
  avisoLectura,
  avisoPrevio,
  avisosParaRevisar,
  estadoFicha,
  parsearNumero,
  pasaFiltro,
  textoCruce,
  textoNumero,
} from "./ConteoGafetes.logica";

const disponible = (numero: number): GafeteParaConteo => ({
  numero,
  estado: "Disponible",
  en_uso_por: null,
});

describe("estadoFicha", () => {
  it("pinta cada gafete según lo esperado y lo contado", () => {
    expect(estadoFicha(disponible(1), false)).toBe("pendiente");
    expect(estadoFicha(disponible(1), true)).toBe("contado");
    const asignado = { ...disponible(2), en_uso_por: "Ana" };
    expect(estadoFicha(asignado, false)).toBe("asignado");
    expect(estadoFicha(asignado, true)).toBe("revisar");
    const perdido: GafeteParaConteo = { ...disponible(3), estado: "Perdido" };
    expect(estadoFicha(perdido, false)).toBe("perdido");
    expect(estadoFicha(perdido, true)).toBe("revisar");
  });

  it("el filtro de faltantes sólo deja los pendientes", () => {
    expect(pasaFiltro("pendiente", "faltan")).toBe(true);
    expect(pasaFiltro("asignado", "faltan")).toBe(false);
    expect(pasaFiltro("revisar", "contados")).toBe(true);
    expect(pasaFiltro("perdido", "todos")).toBe(true);
  });
});

describe("agruparTablero", () => {
  it("con pocos gafetes no agrupa y ordena por número", () => {
    const grupos = agruparTablero([disponible(3), disponible(1), disponible(2)]);
    expect(grupos).toHaveLength(1);
    expect(grupos[0].titulo).toBeNull();
    expect(grupos[0].gafetes.map((g) => g.numero)).toEqual([1, 2, 3]);
  });

  it("con muchos gafetes agrupa por centenas", () => {
    const gafetes = Array.from({ length: 150 }, (_, i) => disponible(i + 1));
    const grupos = agruparTablero(gafetes);
    expect(grupos.map((g) => g.titulo)).toEqual(["001–100", "101–200"]);
    expect(grupos[0].gafetes).toHaveLength(100);
    expect(grupos[1].gafetes).toHaveLength(50);
  });
});

describe("avisosParaRevisar", () => {
  it("un asignado que es parte de un cruce sale una sola vez", () => {
    const resultado: ResultadoConteo = {
      esperados: 3,
      presentes: [5],
      faltantes: [9],
      en_uso: [],
      en_uso_contados: [
        { numero: 6, nombre: "Juan" },
        { numero: 7, nombre: "Ana" },
      ],
      no_existen: [],
      perdidos_contados: [12],
      perdidos: [],
      posibles_cruces: [{ contado: { numero: 6, nombre: "Juan" }, faltante: 9, parecido: "seis_nueve" }],
    };
    const avisos = avisosParaRevisar(resultado);
    expect(avisos).toHaveLength(3);
    expect(avisos[0]).toContain("pudo digitarse el 06 en vez del 09");
    expect(avisos[1]).toContain("El 07 figura asignado a Ana");
    expect(avisos[2]).toContain("El 12 figura perdido");
  });
});

describe("textoCruce", () => {
  it("dice qué se contó, qué falta y a quién revisar", () => {
    expect(
      textoCruce({
        contado: { numero: 6, nombre: "Juan Pérez" },
        faltante: 9,
        parecido: "seis_nueve",
      }),
    ).toBe(
      "El 06 figura asignado a Juan Pérez y se contó; falta el 09. Revise qué gafete se le entregó a Juan Pérez: pudo digitarse el 06 en vez del 09.",
    );
  });
});

describe("parsearNumero", () => {
  it("acepta números positivos y rechaza lo demás", () => {
    expect(parsearNumero(" 7 ")).toBe(7);
    expect(parsearNumero("07")).toBe(7);
    expect(parsearNumero("0")).toBeNull();
    expect(parsearNumero("-3")).toBeNull();
    expect(parsearNumero("7a")).toBeNull();
    expect(parsearNumero("")).toBeNull();
  });
});

describe("textoNumero", () => {
  it("muestra dos dígitos como mínimo", () => {
    expect(textoNumero(7)).toBe("07");
    expect(textoNumero(125)).toBe("125");
  });
});

describe("avisoPrevio", () => {
  it("rechaza lo que no es número y lo repetido", () => {
    expect(avisoPrevio("abc", [])).toMatchObject({ nivel: "error", anotar: false });
    expect(avisoPrevio("7", [3, 7])).toEqual({
      nivel: "error",
      texto: "El 07 ya lo había colocado.",
      anotar: false,
    });
  });

  it("deja pasar un número nuevo para preguntarle al catálogo", () => {
    expect(avisoPrevio("12", [3, 7])).toBeNull();
  });
});

describe("avisoLectura", () => {
  it("anota lo que se tiene en la mano y rechaza lo que no existe", () => {
    expect(avisoLectura(3, { tipo: "esperado" })).toMatchObject({ nivel: "bien", anotar: true });
    expect(avisoLectura(99, { tipo: "no_existe" })).toEqual({
      nivel: "error",
      texto: "El 99 no existe en el catálogo.",
      anotar: false,
    });
    expect(avisoLectura(5, { tipo: "perdido" })).toMatchObject({ nivel: "atencion", anotar: true });
    expect(avisoLectura(4, { tipo: "en_uso", nombre: "Juan Pérez" })).toMatchObject({
      nivel: "atencion",
      anotar: true,
    });
    expect(avisoLectura(4, { tipo: "en_uso", nombre: "Juan Pérez" }).texto).toContain("Juan Pérez");
    // Lo primero que se pide revisar es la digitación.
    expect(avisoLectura(4, { tipo: "en_uso", nombre: "Juan Pérez" }).texto).toContain("bien digitado");
    expect(avisoLectura(5, { tipo: "perdido" }).texto).toContain("bien digitado");
  });
});
