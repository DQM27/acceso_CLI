import { describe, expect, it } from "vitest";
import type { Fila } from "./calculo";
import { calcularCalendario, calcularCalor, nivel, rangosDeNiveles } from "./mapas";

const ingresos: Fila[] = [
  { dia: "2026-10-19", dia_semana: "Lunes", hora: 7, empresa: "A" },
  { dia: "2026-10-19", dia_semana: "Lunes", hora: 7, empresa: "A" },
  { dia: "2026-10-19", dia_semana: "Lunes", hora: 9, empresa: "B" },
  { dia: "2026-10-23", dia_semana: "Viernes", hora: 10, empresa: "A" },
  { dia: "2026-10-25", dia_semana: "Domingo", hora: null, empresa: null },
];

describe("niveles de color", () => {
  it("cinco niveles de menos a más; 0 sólo si está vacío", () => {
    expect([0, 1, 2, 5, 6, 10].map((v) => nivel(v, 10))).toEqual([0, 1, 1, 3, 3, 5]);
    expect(nivel(3, 0)).toBe(0);
  });

  it("la leyenda dice qué rango cubre cada nivel, sin niveles vacíos con máximos chicos", () => {
    expect(rangosDeNiveles(10)).toEqual([
      { nivel: 1, texto: "1–2" },
      { nivel: 2, texto: "3–4" },
      { nivel: 3, texto: "5–6" },
      { nivel: 4, texto: "7–8" },
      { nivel: 5, texto: "9–10" },
    ]);
    expect(rangosDeNiveles(2)).toEqual([
      { nivel: 3, texto: "1" },
      { nivel: 5, texto: "2" },
    ]);
    expect(rangosDeNiveles(0)).toEqual([]);
  });
});

describe("calcularCalor", () => {
  it("horas pico: los siete días en orden y el día completo de 0 a 23 h; la celda cuenta sus ingresos", () => {
    const d = calcularCalor("ingresos", { tipo: "calor", filas: "dia_semana", columnas: "hora", medida: "contar" }, ingresos);
    expect(d.filas.map((f) => f.clave)).toEqual(["Lunes", "Martes", "Miércoles", "Jueves", "Viernes", "Sábado", "Domingo"]);
    // Las 24 horas aunque sólo haya datos de 7 a 10; el vacío al final.
    expect(d.columnas.map((c) => c.clave)).toEqual([...Array.from({ length: 24 }, (_, i) => String(i)), "(sin dato)"]);
    expect(d.columnas[7].etiqueta).toBe("7 h");
    const fila = (valores: Record<number, number>) => [...Array.from({ length: 24 }, (_, i) => valores[i] ?? 0), valores[24] ?? 0];
    expect(d.valores[0]).toEqual(fila({ 7: 2, 9: 1 }));
    expect(d.valores[4]).toEqual(fila({ 10: 1 }));
    expect(d.valores[6]).toEqual(fila({ 24: 1 }));
    expect(d.maximo).toBe(2);
  });

  it("con textos, los mayores primero y lo que no entra en 'Otros'; las fechas en orden y sin huecos", () => {
    const muchas: Fila[] = Array.from({ length: 20 }, (_, i) => ({ empresa: `E${String(i).padStart(2, "0")}`, dia: "2026-10-19" }));
    muchas.push({ empresa: "E05", dia: "2026-10-21" });
    const d = calcularCalor("ingresos", { tipo: "calor", filas: "empresa", columnas: "dia", medida: "contar" }, muchas);
    expect(d.filas[0].clave).toBe("E05");
    expect(d.filas).toHaveLength(16);
    expect(d.filas.at(-1)?.clave).toBe("Otros");
    expect(d.columnas.map((c) => c.clave)).toEqual(["2026-10-19", "2026-10-20", "2026-10-21"]);
    // Las 5 empresas que no entraron suman en "Otros".
    expect(d.valores.at(-1)).toEqual([5, 0, 0]);
  });
});

describe("calcularCalendario", () => {
  const vencimientos: Fila[] = [
    { vencimiento: "2026-11-03" },
    { vencimiento: "2026-11-03" },
    { vencimiento: "2026-11-30" },
    { vencimiento: "2027-02-14" },
    { vencimiento: "2027-06-01" },
    { vencimiento: null },
  ];

  it("de un mes a otro, con cada día en su columna (lunes primero) y su valor", () => {
    const d = calcularCalendario({ tipo: "calendario", fecha: "vencimiento", medida: "contar", meses: 6 }, vencimientos, "2026-11-03");
    expect(d.meses.map((m) => m.clave)).toEqual(["2026-11", "2026-12", "2027-01", "2027-02", "2027-03", "2027-04"]);
    const noviembre = d.meses[0];
    expect(noviembre.etiqueta).toMatch(/noviembre.*2026/);
    expect(noviembre.dias).toHaveLength(30);
    // El 1 de noviembre de 2026 es domingo; el 3, martes.
    expect(noviembre.dias[0]).toMatchObject({ numero: 1, diaSemana: 6, semana: 0 });
    expect(noviembre.dias[2]).toMatchObject({ fecha: "2026-11-03", diaSemana: 1, semana: 1, valor: 2 });
    expect(d.maximo).toBe(2);
    expect(d.hoy).toBe("2026-11-03");
  });

  it("avisa cuántas quedan después de los meses que se ven y cuántas no tienen fecha", () => {
    const d = calcularCalendario({ tipo: "calendario", fecha: "vencimiento", medida: "contar", meses: 6 }, vencimientos);
    expect(d.fuera).toBe(1);
    expect(d.sinFecha).toBe(1);
    const corto = calcularCalendario({ tipo: "calendario", fecha: "vencimiento", medida: "contar", meses: 2 }, vencimientos);
    expect(corto.meses).toHaveLength(2);
    expect(corto.fuera).toBe(2);
  });

  it("sin fechas no hay meses", () => {
    expect(calcularCalendario({ tipo: "calendario", fecha: "vencimiento", medida: "contar" }, [{ vencimiento: null }]).meses).toEqual([]);
  });
});
