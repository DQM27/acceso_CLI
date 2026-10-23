import { describe, expect, it } from "vitest";
import { MAXIMO_BARRAS, calcularBarras, calcularKpi, calcularTabla, textoValor, type Fila } from "./calculo";

const filas: Fila[] = [
  { nombre: "Ana", empresa: "A", vencimiento: "2026-10-05", dias_para_vencer: 3, acceso_activo: true },
  { nombre: "Beto", empresa: "A", vencimiento: "2026-10-20", dias_para_vencer: 18, acceso_activo: true },
  { nombre: "Cris", empresa: "B", vencimiento: null, dias_para_vencer: null, acceso_activo: false },
  { nombre: "Ana", empresa: null, vencimiento: "2026-11-01", dias_para_vencer: 30, acceso_activo: true },
];

describe("calcularKpi", () => {
  const kpi = (medida: "contar" | "contar_distintos" | "minimo" | "maximo", campo?: string) =>
    calcularKpi({ tipo: "kpi", titulo: "t", medida, campo }, filas);

  it("cuenta filas y valores distintos (sin contar los vacíos)", () => {
    expect(kpi("contar")).toBe(4);
    expect(kpi("contar_distintos", "nombre")).toBe(3);
    expect(kpi("contar_distintos", "empresa")).toBe(2);
  });

  it("mínimo y máximo de fechas y números, ignorando vacíos", () => {
    expect(kpi("minimo", "vencimiento")).toBe("2026-10-05");
    expect(kpi("maximo", "vencimiento")).toBe("2026-11-01");
    expect(kpi("minimo", "dias_para_vencer")).toBe(3);
    expect(kpi("maximo", "dias_para_vencer")).toBe(30);
  });

  it("sin filas: cuenta 0 y mínimo/máximo vacíos", () => {
    expect(calcularKpi({ tipo: "kpi", titulo: "t", medida: "contar" }, [])).toBe(0);
    expect(calcularKpi({ tipo: "kpi", titulo: "t", medida: "maximo", campo: "vencimiento" }, [])).toBeNull();
  });
});

describe("calcularBarras", () => {
  it("agrupa por el eje y ordena de mayor a menor, con '(sin dato)' para los vacíos", () => {
    const barras = calcularBarras("contratistas", { tipo: "barras", eje_x: "empresa", medida: "contar" }, filas);
    expect(barras).toEqual([
      { etiqueta: "A", valor: 2 },
      { etiqueta: "(sin dato)", valor: 1 },
      { etiqueta: "B", valor: 1 },
    ]);
  });

  it("un eje de fecha va en orden cronológico", () => {
    const ingresos: Fila[] = [{ dia: "2026-10-12" }, { dia: "2026-10-10" }, { dia: "2026-10-12" }];
    expect(calcularBarras("ingresos", { tipo: "barras", eje_x: "dia", medida: "contar" }, ingresos)).toEqual([
      { etiqueta: "2026-10-10", valor: 1 },
      { etiqueta: "2026-10-12", valor: 2 },
    ]);
  });

  it("mide distintos por grupo", () => {
    const barras = calcularBarras(
      "contratistas",
      { tipo: "barras", eje_x: "empresa", medida: "contar_distintos", campo: "nombre" },
      filas,
    );
    expect(barras[0]).toEqual({ etiqueta: "A", valor: 2 });
  });

  it(`muestra como máximo ${MAXIMO_BARRAS} barras`, () => {
    const muchas: Fila[] = Array.from({ length: 50 }, (_, i) => ({ empresa: `E${i}` }));
    expect(calcularBarras("contratistas", { tipo: "barras", eje_x: "empresa", medida: "contar" }, muchas)).toHaveLength(
      MAXIMO_BARRAS,
    );
  });
});

describe("calcularTabla", () => {
  it("elige columnas, ordena (vacíos al final) y pone etiquetas del catálogo", () => {
    const t = calcularTabla(
      "contratistas",
      { tipo: "tabla", columnas: ["nombre", "dias_para_vencer"], orden: { campo: "dias_para_vencer", dir: "asc" } },
      filas,
    );
    expect(t.columnas).toEqual([
      { id: "nombre", etiqueta: "Nombre" },
      { id: "dias_para_vencer", etiqueta: "Días para vencer" },
    ]);
    expect(t.filas.map((f) => f.nombre)).toEqual(["Ana", "Beto", "Ana", "Cris"]);
    expect(t.filas.map((f) => f.dias_para_vencer)).toEqual([3, 18, 30, null]);
  });

  it("orden descendente deja los vacíos igualmente al final", () => {
    const t = calcularTabla(
      "contratistas",
      { tipo: "tabla", columnas: ["nombre"], orden: { campo: "dias_para_vencer", dir: "desc" } },
      filas,
    );
    expect(t.filas.map((f) => f.dias_para_vencer ?? null)).toEqual([30, 18, 3, null]);
  });

  it("agrupada: una fila por grupo con su cantidad, los grupos grandes primero", () => {
    const t = calcularTabla("contratistas", { tipo: "tabla", columnas: ["empresa"], agrupar_por: "empresa" }, filas);
    expect(t.columnas.map((c) => c.etiqueta)).toEqual(["Empresa", "Cantidad"]);
    expect(t.filas[0]).toEqual({ empresa: "A", cantidad: 2 });
    expect(t.total).toBe(3);
  });

  it("agrupada y ordenada por 'cantidad' ascendente", () => {
    const t = calcularTabla(
      "contratistas",
      { tipo: "tabla", columnas: ["empresa"], agrupar_por: "empresa", orden: { campo: "cantidad", dir: "asc" } },
      filas,
    );
    expect(t.filas.at(-1)).toEqual({ empresa: "A", cantidad: 2 });
  });

  it("recorta por el límite y avisa cuántas había", () => {
    const t = calcularTabla("contratistas", { tipo: "tabla", columnas: ["nombre"], limite: 2 }, filas);
    expect(t.filas).toHaveLength(2);
    expect(t.total).toBe(4);
  });
});

describe("textoValor", () => {
  it("formatea vacíos y booleanos", () => {
    expect(textoValor(null)).toBe("—");
    expect(textoValor(undefined)).toBe("—");
    expect(textoValor(true)).toBe("Sí");
    expect(textoValor(false)).toBe("No");
    expect(textoValor(7)).toBe("7");
  });
});
