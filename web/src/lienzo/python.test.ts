// @vitest-environment node
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";
import { PREPARACION } from "../../../sandbox-lienzo/public/python-motor.js";
import { EJEMPLO_ANALISIS, EJEMPLO_PANDAS } from "../../../supabase/functions/lienzo-ia/instrucciones.ts";
import { REAL_CONTRA_NORMAL } from "./vistas";

/**
 * El envoltorio de Python del panel "analisis" (sandbox-lienzo/public/python-motor.js)
 * corrido con el Pyodide de verdad, el mismo que publica el sandbox. Pyodide no
 * está en git: si no se bajó (`node sandbox-lienzo/descargar-pyodide.mjs`), se salta.
 */
const CARPETA = resolve(__dirname, "../../../sandbox-lienzo/public/pyodide");
const hayPyodide = existsSync(resolve(CARPETA, "pyodide.asm.wasm"));

interface Pyodide {
  runPython(codigo: string): unknown;
  loadPackage(nombre: string, opciones?: object): Promise<unknown>;
  globals: { get(nombre: string): (...args: unknown[]) => unknown };
}

let py: Pyodide;
type Respuesta = { filas?: Record<string, unknown>[]; error?: string };

function correr(codigo: string, datos: unknown[], salida: string[], columnas: unknown[] = []): Respuesta {
  const ejecutar = py.globals.get("_ejecutar");
  return JSON.parse(ejecutar(codigo, JSON.stringify(datos), JSON.stringify(columnas), JSON.stringify(salida)) as string);
}

const ingresos = [
  { dia: "2026-10-12", empresa: "A" },
  { dia: "2026-10-12", empresa: "B" },
  { dia: "2026-10-13", empresa: "A" },
  { dia: "2026-10-14", empresa: "A" },
  { dia: "2026-10-14", empresa: "A" },
  { dia: "2026-10-14", empresa: "B" },
];

describe.skipIf(!hayPyodide)("Python del panel análisis (Pyodide)", () => {
  beforeAll(async () => {
    const { loadPyodide } = await import(/* @vite-ignore */ pathToFileURL(resolve(CARPETA, "pyodide.mjs")).href);
    py = await loadPyodide({ indexURL: `${CARPETA}/`, stdout: () => {}, stderr: () => {} });
    py.runPython(PREPARACION);
  }, 60_000);

  it("recibe las filas en «datos» y devuelve «resultado» con las columnas declaradas", () => {
    const codigo = `
from collections import Counter
por_dia = Counter(f["dia"] for f in datos)
resultado = [{"dia": d, "cantidad": n, "sobra": 1} for d, n in sorted(por_dia.items())]
`;
    expect(correr(codigo, ingresos, ["dia", "cantidad"])).toEqual({
      filas: [
        { dia: "2026-10-12", cantidad: 2 },
        { dia: "2026-10-13", cantidad: 1 },
        { dia: "2026-10-14", cantidad: 3 },
      ],
    });
  });

  it("estadística de la biblioteca estándar: una regresión lineal para pronosticar", () => {
    const codigo = `
import statistics
x = [0, 1, 2, 3]
y = [10, 12, 14, 16]
pendiente, corte = statistics.linear_regression(x, y)
resultado = [{"dia": i, "pronostico": round(corte + pendiente * i, 2)} for i in range(4, 6)]
`;
    expect(correr(codigo, [], ["dia", "pronostico"]).filas).toEqual([
      { dia: 4, pronostico: 18 },
      { dia: 5, pronostico: 20 },
    ]);
  });

  it("convierte fechas, decimales y valores no finitos a algo que el panel puede mostrar", () => {
    const codigo = `
import datetime, decimal, math
resultado = [{"fecha": datetime.date(2026, 10, 23), "monto": decimal.Decimal("1.5"), "raro": math.nan, "ok": True}]
`;
    expect(correr(codigo, [], ["fecha", "monto", "raro", "ok"]).filas).toEqual([{ fecha: "2026-10-23", monto: 1.5, raro: null, ok: true }]);
  });

  it("no deja importar el puente a JavaScript, la red ni el intérprete", () => {
    for (const modulo of ["js", "pyodide", "micropip", "urllib.request", "os", "sys", "importlib"]) {
      const r = correr(`import ${modulo}\nresultado = []`, [], ["x"]);
      expect(r.error, modulo).toMatch(/no se puede usar/);
    }
    expect(correr("resultado = [{'x': eval('1+1')}]", [], ["x"]).error).toMatch(/NameError/);
    expect(correr("open('/tmp/a', 'w')\nresultado = []", [], ["x"]).error).toMatch(/NameError/);
  });

  it("los errores dicen qué pasó y en qué línea, sin cortar el motor", () => {
    expect(correr("x = 1\ny = 1 / 0\nresultado = []", [], ["x"]).error).toBe("ZeroDivisionError (línea 2): division by zero");
    expect(correr("resultado = [", [], ["x"]).error).toMatch(/^Error de sintaxis en la línea 1/);
    expect(correr("x = 1", [], ["x"]).error).toMatch(/no dejó nada en «resultado»/);
    expect(correr("resultado = 5", [], ["x"]).error).toMatch(/tiene que ser una lista/);
    expect(correr("resultado = [{'y': 1}]", [], ["x"]).error).toMatch(/le faltan las columnas: x/);
    expect(correr("resultado = [{'x': i} for i in range(2001)]", [], ["x"]).error).toMatch(/máximo es 2000/);
    // Después de un error sigue andando.
    expect(correr("resultado = {'x': 1}", [], ["x"]).filas).toEqual([{ x: 1 }]);
  });

  it("el ejemplo de las instrucciones del modelo: lo real día por día y 7 días de pronóstico por día de la semana", () => {
    // Dos semanas: los lunes (13 y 20) tuvieron 4 y 2 ingresos; los demás días, 1.
    const datos: { dia: string }[] = [];
    for (let i = 0; i < 14; i++) {
      const dia = new Date(Date.UTC(2026, 9, 12 + i)).toISOString().slice(0, 10);
      const veces = dia === "2026-10-19" ? 4 : dia === "2026-10-12" ? 2 : 1;
      for (let j = 0; j < veces; j++) datos.push({ dia });
    }
    const r = correr(EJEMPLO_ANALISIS, datos, ["dia", "cantidad", "serie"]);
    expect(r.error).toBeUndefined();
    const filas = r.filas ?? [];
    const reales = filas.filter((f) => f.serie === "Real");
    const pronostico = filas.filter((f) => f.serie === "Pronóstico");
    expect(reales).toHaveLength(14);
    expect(pronostico.map((f) => f.dia)).toEqual(["2026-10-26", "2026-10-27", "2026-10-28", "2026-10-29", "2026-10-30", "2026-10-31", "2026-11-01"]);
    // El lunes 26 se espera el promedio de los lunes (2 y 4 → 3); el resto, 1.
    expect(pronostico.map((f) => f.cantidad)).toEqual([3, 1, 1, 1, 1, 1, 1]);
    expect(correr(EJEMPLO_ANALISIS, [], ["dia", "cantidad", "serie"])).toEqual({ filas: [] });
  });

  it("numpy se carga sólo si el código lo importa, y sus números vuelven como números", async () => {
    const paquetes = py.globals.get("_paquetes");
    const conNumpy = paquetes("import numpy as np\nimport statistics") as { toJs(): string[]; destroy(): void };
    expect(conNumpy.toJs()).toEqual(["numpy"]);
    conNumpy.destroy();
    await py.loadPackage("numpy", { messageCallback: () => {}, errorCallback: () => {} });
    const codigo = `
import numpy as np
valores = np.array([f["n"] for f in datos], dtype=float)
resultado = [{"promedio": valores.mean(), "desvio": round(float(valores.std()), 3), "maximo": valores.max()}]
`;
    expect(correr(codigo, [{ n: 2 }, { n: 4 }, { n: 6 }], ["promedio", "desvio", "maximo"]).filas).toEqual([
      { promedio: 4, desvio: 1.633, maximo: 6 },
    ]);
  }, 60_000);

  it("pandas: los ejemplos encuentran el día fuera de lo normal y un DataFrame vuelve como filas", async () => {
    const paquetes = py.globals.get("_paquetes");
    const conPandas = paquetes(EJEMPLO_PANDAS) as { toJs(): string[]; destroy(): void };
    expect(conPandas.toJs()).toEqual(["pandas"]);
    conPandas.destroy();
    await py.loadPackage("pandas", { messageCallback: () => {}, errorCallback: () => {} });

    // Tres semanas con 10 ingresos por día (salvo el domingo 18, sin ingresos); el miércoles 21, 30.
    const datos: { dia: string }[] = [];
    for (let i = 0; i < 21; i++) {
      const dia = new Date(Date.UTC(2026, 9, 5 + i)).toISOString().slice(0, 10);
      const veces = dia === "2026-10-21" ? 30 : dia === "2026-10-18" ? 0 : 10;
      for (let j = 0; j < veces; j++) datos.push({ dia });
    }
    expect(correr(EJEMPLO_PANDAS, datos, ["dia", "cantidad", "esperado", "diferencia"])).toEqual({
      filas: [
        { dia: "2026-10-21", cantidad: 30, esperado: 10, diferencia: 20 },
        { dia: "2026-10-18", cantidad: 0, esperado: 10, diferencia: -10 },
      ],
    });
    expect(correr(EJEMPLO_PANDAS, [], ["dia", "cantidad", "esperado", "diferencia"])).toEqual({ filas: [] });

    const r = correr(REAL_CONTRA_NORMAL, datos, ["dia", "cantidad", "serie"]);
    const filas = r.filas ?? [];
    expect(filas.filter((f) => f.serie === "Real")).toHaveLength(21);
    expect(filas.find((f) => f.serie === "Lo normal" && f.dia === "2026-10-21")?.cantidad).toBe(10);
    // Los vacíos de pandas vuelven vacíos.
    expect(correr("import pandas as pd\nresultado = pd.DataFrame({'a': [pd.NA], 'b': [pd.NaT]})", [], ["a", "b"]).filas).toEqual([
      { a: null, b: null },
    ]);
  }, 60_000);
});
