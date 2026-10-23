import { describe, expect, it } from "vitest";
import { OTROS, SIN_DATO, calcularTendencia, diasEntre, sumarDias, variacion } from "./calculo";
import { conFiltro, describirFiltro, filtroParaCategoria, sinFiltro } from "./edicion";
import { validarLienzo, type Lienzo } from "./esquema";

describe("filtroParaCategoria (clic para profundizar)", () => {
  it("texto y enumeraciones: 'igual' a lo que se tocó", () => {
    expect(filtroParaCategoria("contratistas", "empresa", "ACME")).toEqual({ campo: "empresa", op: "igual", valor: "ACME" });
    expect(filtroParaCategoria("contratistas", "estado_praind", "VENCIDA")).toEqual({
      campo: "estado_praind",
      op: "igual",
      valor: "VENCIDA",
    });
  });

  it("fechas: el día, o la semana o el mes enteros", () => {
    expect(filtroParaCategoria("ingresos", "dia", "2026-10-22", "dia")).toEqual({ campo: "dia", op: "igual", valor: "2026-10-22" });
    expect(filtroParaCategoria("ingresos", "dia", "2026-10-19", "semana")).toEqual({
      campo: "dia",
      op: "entre",
      valor: ["2026-10-19", "2026-10-25"],
    });
    expect(filtroParaCategoria("ingresos", "dia", "2026-02", "mes")).toEqual({
      campo: "dia",
      op: "entre",
      valor: ["2026-02-01", "2026-02-28"],
    });
  });

  it("no profundiza en 'Otros', '(sin dato)', valores fuera del catálogo ni campos que sólo buscan texto", () => {
    expect(filtroParaCategoria("contratistas", "empresa", OTROS)).toBeNull();
    expect(filtroParaCategoria("contratistas", "empresa", SIN_DATO)).toBeNull();
    expect(filtroParaCategoria("contratistas", "estado_praind", "INVENTADO")).toBeNull();
    expect(filtroParaCategoria("ingresos", "nombre", "ANA")).toBeNull();
    expect(filtroParaCategoria("contratistas", "cedula", "1")).toBeNull();
  });

  it("booleanos y números", () => {
    expect(filtroParaCategoria("contratistas", "acceso_activo", "No")).toEqual({ campo: "acceso_activo", op: "igual", valor: false });
    expect(filtroParaCategoria("contratistas", "dias_para_vencer", "7")).toEqual({ campo: "dias_para_vencer", op: "igual", valor: 7 });
  });

  it("el filtro resultante siempre pasa el validador", () => {
    const base: Lienzo = { fuente: "ingresos", filtros: [], widgets: [{ tipo: "kpi", titulo: "t", medida: "contar" }] };
    for (const [campo, clave, agrupacion] of [
      ["empresa", "ACME", null],
      ["sitio", "Brisas", null],
      ["tipo_ingreso", "SWAT", null],
      ["dia", "2026-10-19", "semana"],
    ] as const) {
      const filtro = filtroParaCategoria("ingresos", campo, clave, agrupacion);
      if (!filtro) throw new Error(`Sin filtro para ${campo}`);
      expect(validarLienzo(conFiltro(base, filtro)).ok).toBe(true);
    }
  });
});

describe("conFiltro / sinFiltro", () => {
  const base: Lienzo = {
    fuente: "contratistas",
    filtros: [
      { campo: "empresa", op: "no_en", valor: ["A"] },
      { campo: "estado_praind", op: "igual", valor: "VENCIDA" },
    ],
    widgets: [{ tipo: "kpi", titulo: "t", medida: "contar" }],
  };

  it("un filtro nuevo reemplaza al que había sobre el mismo campo (no se pisan)", () => {
    const nuevo = conFiltro(base, { campo: "empresa", op: "igual", valor: "ACME" });
    expect(nuevo.filtros).toEqual([
      { campo: "estado_praind", op: "igual", valor: "VENCIDA" },
      { campo: "empresa", op: "igual", valor: "ACME" },
    ]);
    // El original no cambia: el historial de Deshacer depende de eso.
    expect(base.filtros).toHaveLength(2);
  });

  it("quita un filtro por su posición", () => {
    expect(sinFiltro(base, 0).filtros).toEqual([{ campo: "estado_praind", op: "igual", valor: "VENCIDA" }]);
  });
});

describe("describirFiltro", () => {
  it("texto corto para los chips", () => {
    expect(describirFiltro("contratistas", { campo: "empresa", op: "igual", valor: "ACME" })).toBe("Empresa: ACME");
    expect(describirFiltro("contratistas", { campo: "empresa", op: "no_en", valor: ["A", "B"] })).toBe("Empresa: sin A, B");
    expect(describirFiltro("contratistas", { campo: "nombre", op: "contiene", valor: "ana" })).toBe('Nombre: contiene "ana"');
    expect(describirFiltro("contratistas", { campo: "acceso_activo", op: "igual", valor: true })).toBe("Acceso activo: Sí");
    expect(describirFiltro("ingresos", { campo: "dia", op: "entre", valor: ["2026-10-01", "2026-10-31"] })).toBe(
      "Día: 01/10/2026 a 31/10/2026",
    );
  });
});

describe("comparación y tendencia", () => {
  it("variación contra el período anterior; sin base no hay porcentaje", () => {
    expect(variacion(112, 100)).toBeCloseTo(0.12);
    expect(variacion(80, 100)).toBeCloseTo(-0.2);
    expect(variacion(5, 0)).toBeNull();
  });

  it("fechas: sumar días y días entre", () => {
    expect(sumarDias("2026-10-22", -13)).toBe("2026-10-09");
    expect(sumarDias("2026-12-31", 1)).toBe("2027-01-01");
    expect(diasEntre("2026-10-09", "2026-10-22")).toBe(13);
  });

  it("la tendencia tiene un valor por día del rango, 0 en los días sin datos", () => {
    const filas = [{ dia: "2026-10-20" }, { dia: "2026-10-20" }, { dia: "2026-10-22" }];
    expect(calcularTendencia({ tipo: "kpi", titulo: "t", medida: "contar" }, filas, "2026-10-19", "2026-10-22")).toEqual([0, 2, 0, 1]);
  });
});
