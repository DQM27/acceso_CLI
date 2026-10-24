import { describe, expect, it } from "vitest";
import type { Fila } from "./calculo";
import type { WidgetTabla } from "./esquema";
import { compararValores, filtrarPorBusqueda, modeloTabla, textoCelda, tonoCelda } from "./TablaRica.logica";

const filas: Fila[] = [
  { nombre: "Ana", empresa: "A", vencimiento: "2026-10-05", dias_para_vencer: 3, estado_praind: "POR_VENCER", acceso_activo: true },
  { nombre: "Beto", empresa: "A", vencimiento: "2026-10-20", dias_para_vencer: 18, estado_praind: "POR_VENCER", acceso_activo: true },
  { nombre: "Cris", empresa: "B", vencimiento: null, dias_para_vencer: null, estado_praind: "SIN_REGISTRO", acceso_activo: false },
  { nombre: "Sánchez", empresa: null, vencimiento: "2026-11-01", dias_para_vencer: -40, estado_praind: "VENCIDA", acceso_activo: true },
];

const tabla = (resto: Partial<WidgetTabla>): WidgetTabla => ({ tipo: "tabla", columnas: ["nombre"], ...resto });

describe("modeloTabla sin agrupar", () => {
  it("columnas con su etiqueta y tipo del catálogo; ordena por el valor real con los vacíos al final", () => {
    const m = modeloTabla("contratistas", tabla({ columnas: ["nombre", "dias_para_vencer"], orden: { campo: "dias_para_vencer", dir: "asc" } }), filas);
    expect(m.columnas.map((c) => [c.etiqueta, c.tipo])).toEqual([
      ["Nombre", "texto"],
      ["Días para vencer", "numero"],
    ]);
    expect(m.filas.map((f) => f.dias_para_vencer)).toEqual([-40, 3, 18, null]);
  });

  it("en descendente los vacíos también quedan al final", () => {
    const m = modeloTabla("contratistas", tabla({ orden: { campo: "vencimiento", dir: "desc" } }), filas);
    expect(m.filas.map((f) => f.vencimiento)).toEqual(["2026-11-01", "2026-10-20", "2026-10-05", null]);
  });

  it("recorta por el límite y dice cuántas había", () => {
    const m = modeloTabla("contratistas", tabla({ limite: 2 }), filas);
    expect(m.filas).toHaveLength(2);
    expect(m.total).toBe(4);
  });

  it("las columnas con barra saben el mayor valor (los negativos no cuentan)", () => {
    const m = modeloTabla("contratistas", tabla({ columnas: ["nombre", "dias_para_vencer"], barras: ["dias_para_vencer"] }), filas);
    expect(m.columnas[1].maximoBarra).toBe(18);
    expect(m.columnas[0].maximoBarra).toBeUndefined();
  });

  it("busca sin tildes ni mayúsculas, en el texto como se ve (estados y días en palabras)", () => {
    const columnas = ["nombre", "estado_praind", "dias_para_vencer"];
    expect(modeloTabla("contratistas", tabla({ columnas }), filas, { busqueda: "sanchez" }).filas.map((f) => f.nombre)).toEqual(["Sánchez"]);
    expect(modeloTabla("contratistas", tabla({ columnas }), filas, { busqueda: "por vencer" }).total).toBe(2);
    expect(modeloTabla("contratistas", tabla({ columnas }), filas, { busqueda: "vencido hace" }).total).toBe(1);
    expect(modeloTabla("contratistas", tabla({ columnas }), filas, { busqueda: "  " }).total).toBe(4);
  });
});

describe("modeloTabla agrupada", () => {
  const agrupada = tabla({ columnas: ["empresa", "nombre", "vencimiento"], agrupar_por: "empresa" });

  it("un grupo por valor con su cantidad y sus filas; los grandes primero; el detalle sin la columna del grupo", () => {
    const m = modeloTabla("contratistas", agrupada, filas);
    expect(m.agrupada?.campo.etiqueta).toBe("Empresa");
    expect(m.columnas.map((c) => c.id)).toEqual(["nombre", "vencimiento"]);
    expect(m.grupos.map((g) => [g.valor, g.cantidad])).toEqual([
      ["A", 2],
      ["B", 1],
      [null, 1],
    ]);
    expect(m.grupos[0].filas.map((f) => f.nombre)).toEqual(["Ana", "Beto"]);
    expect(m.agrupada?.cantidad.maximoBarra).toBe(2);
    expect(m.total).toBe(3);
  });

  it("ordena los grupos por cantidad ascendente o por su nombre, y las filas de cada grupo por su campo", () => {
    expect(modeloTabla("contratistas", { ...agrupada, orden: { campo: "cantidad", dir: "asc" } }, filas).grupos.at(-1)?.valor).toBe("A");
    expect(modeloTabla("contratistas", { ...agrupada, orden: { campo: "empresa", dir: "desc" } }, filas).grupos.map((g) => g.valor)).toEqual([
      "B",
      "A",
      null,
    ]);
    const porNombre = modeloTabla("contratistas", { ...agrupada, orden: { campo: "nombre", dir: "desc" } }, filas);
    expect(porNombre.grupos[0].filas.map((f) => f.nombre)).toEqual(["Beto", "Ana"]);
  });

  it("la búsqueda filtra las filas y los grupos se recalculan (también busca en el nombre del grupo)", () => {
    expect(modeloTabla("contratistas", agrupada, filas, { busqueda: "beto" }).grupos.map((g) => [g.valor, g.cantidad])).toEqual([["A", 1]]);
    expect(modeloTabla("contratistas", agrupada, filas, { busqueda: "b" }).grupos.map((g) => g.valor)).toContain("B");
  });

  it("con tendencia: cuántas filas hubo cada día, de la primera a la última fecha", () => {
    const ingresos: Fila[] = [
      { empresa: "A", dia: "2026-10-01" },
      { empresa: "A", dia: "2026-10-01" },
      { empresa: "A", dia: "2026-10-03" },
      { empresa: "B", dia: "2026-10-02" },
    ];
    const m = modeloTabla("ingresos", { tipo: "tabla", columnas: ["empresa"], agrupar_por: "empresa", tendencia: "dia" }, ingresos);
    expect(m.agrupada?.conTendencia).toBe(true);
    expect(m.grupos.map((g) => [g.valor, g.tendencia])).toEqual([
      ["A", [2, 0, 1]],
      ["B", [0, 1, 0]],
    ]);
  });

  it("sin tendencia si todas las filas son del mismo día (una línea de un punto no dice nada)", () => {
    const m = modeloTabla("ingresos", { tipo: "tabla", columnas: ["empresa"], agrupar_por: "empresa", tendencia: "dia" }, [{ empresa: "A", dia: "2026-10-01" }]);
    expect(m.agrupada?.conTendencia).toBe(false);
    expect(m.grupos[0].tendencia).toBeUndefined();
  });
});

describe("celdas", () => {
  it("texto: fechas en el formato de la vista, estados y días en palabras, vacíos como raya", () => {
    expect(textoCelda({ id: "vencimiento", tipo: "fecha" }, "2026-10-05", "dia_mes_anio")).toBe("05/10/2026");
    expect(textoCelda({ id: "estado_praind", tipo: "enumeracion" }, "POR_VENCER")).toBe("Por vencer");
    expect(textoCelda({ id: "dias_para_vencer", tipo: "numero" }, -40)).toBe("Vencido hace 40 días");
    expect(textoCelda({ id: "acceso_activo", tipo: "booleano" }, false)).toBe("No");
    expect(textoCelda({ id: "nombre", tipo: "texto" }, null)).toBe("—");
    expect(textoCelda({ id: "n", tipo: "numero" }, 1234)).toBe((1234).toLocaleString("es-CR"));
  });

  it("tono de estado: PRAIND por su estado, días para vencer por lo que falta; lo demás sin etiqueta", () => {
    expect(tonoCelda({ id: "estado_praind" }, "VENCIDA")).toBe("error");
    expect(tonoCelda({ id: "estado_praind" }, "POR_VENCER")).toBe("advertencia");
    expect(tonoCelda({ id: "estado_praind" }, "VIGENTE")).toBe("exito");
    expect(tonoCelda({ id: "dias_para_vencer" }, -1)).toBe("error");
    expect(tonoCelda({ id: "dias_para_vencer" }, 30)).toBe("advertencia");
    expect(tonoCelda({ id: "dias_para_vencer" }, 31)).toBeNull();
    expect(tonoCelda({ id: "empresa" }, "VENCIDA")).toBeNull();
  });

  it("ordenar el estado PRAIND va por gravedad (lo vencido primero), no por orden alfabético", () => {
    const estados = ["VIGENTE", "VENCIDA", null, "POR_VENCER"];
    expect([...estados].sort((a, b) => compararValores("estado_praind", a, b))).toEqual(["VENCIDA", "POR_VENCER", "VIGENTE", null]);
  });

  it("filtrarPorBusqueda exige todas las palabras", () => {
    const columnas = [{ id: "nombre", tipo: "texto" as const }, { id: "empresa", tipo: "texto" as const }];
    expect(filtrarPorBusqueda(filas, columnas, "ana a")).toHaveLength(1);
    expect(filtrarPorBusqueda(filas, columnas, "ana b")).toHaveLength(0);
  });
});
