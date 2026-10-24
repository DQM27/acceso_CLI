import { describe, expect, it } from "vitest";
import { OTROS, SIN_DATO, calcularTendencia, diasEntre, sumarDias, variacion } from "./calculo";
import { conFiltro, describirFiltro, filtroParaCategoria, sinFiltro } from "./edicion";
import { textoComparacion } from "./comparacion";
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

  it("días para vencer en palabras, nunca con signo menos", () => {
    expect(describirFiltro("contratistas", { campo: "dias_para_vencer", op: "entre", valor: [-30, -1] })).toBe(
      "PRAIND: vencidos hace entre 1 y 30 días",
    );
    expect(describirFiltro("contratistas", { campo: "dias_para_vencer", op: "entre", valor: [0, 365] })).toBe(
      "PRAIND: vencen en los próximos 365 días",
    );
    expect(describirFiltro("contratistas", { campo: "dias_para_vencer", op: "igual", valor: -3 })).toBe(
      "PRAIND: vencido hace 3 días",
    );
  });
});

describe("texto de la comparación de un KPI", () => {
  it("en cantidades: más, menos o igual, sin porcentajes engañosos", () => {
    const c = { anterior: 12, contra: "que el jueves pasado a esta hora", modo: "diferencia" as const };
    expect(textoComparacion(7, c)).toBe("5 menos que el jueves pasado a esta hora (12)");
    expect(textoComparacion(15, c)).toBe("3 más que el jueves pasado a esta hora (12)");
    expect(textoComparacion(12, c)).toBe("Igual que el jueves pasado a esta hora (12)");
    expect(textoComparacion(0, c)).not.toMatch(/%|-/);
  });

  it("en porcentaje, en palabras y nunca con signo negativo; sin base, en cantidades", () => {
    const contra = "que los 13 días anteriores";
    expect(textoComparacion(112, { anterior: 100, contra })).toBe("12% más que los 13 días anteriores (100)");
    expect(textoComparacion(52, { anterior: 102, contra })).toBe("49% menos que los 13 días anteriores (102)");
    expect(textoComparacion(0, { anterior: 102, contra })).toBe("100% menos que los 13 días anteriores (102)");
    expect(textoComparacion(40, { anterior: 40, contra })).toBe("Igual que los 13 días anteriores (40)");
    expect(textoComparacion(5, { anterior: 0, contra })).toBe("5 más que los 13 días anteriores (0)");
    for (const actual of [0, 3, 52, 99]) expect(textoComparacion(actual, { anterior: 102, contra })).not.toMatch(/-/);
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
