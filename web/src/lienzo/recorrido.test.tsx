import { act, cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Fila } from "./calculo";
import { CATALOGO, MEDIDAS, definicionCampo, type ColumnaConsulta, type DefinicionCampo, type Fuente } from "./catalogo";
import { validarLienzo, type Lienzo } from "./esquema";
import LienzoVista from "./Lienzo";

/**
 * Recorrido completo del lienzo: arma, a partir del catálogo, todas las
 * combinaciones razonables de cada herramienta (cada campo en cada eje, con y
 * sin serie, cada agrupación de fecha, cada medida…), las que el esquema
 * acepta, y a cada una le hace lo que haría una persona: dibujarla con datos
 * normales, con una sola fila, sin filas y con todo vacío; recorrer sus marcas
 * con el teclado, elegir una para filtrar, volver a dibujarla con menos datos
 * y alternar a la vista de tabla. Si un panel se rompe, `PanelProtegido`
 * muestra su aviso: la prueba junta todos los casos así y los lista.
 *
 * Así no hace falta probar a mano parámetro por parámetro: un campo nuevo en
 * el catálogo entra solo en el recorrido.
 */

// --- Datos ---------------------------------------------------------------------

/** Números pseudoaleatorios repetibles: la misma corrida da los mismos datos. */
function generador(semilla: number) {
  let s = semilla;
  return () => {
    s = (s * 1_103_515_245 + 12_345) % 2_147_483_648;
    return s / 2_147_483_648;
  };
}

const HOY = "2026-10-23";
const TEXTOS: Record<string, string[]> = {
  nombre: ["Ana Pérez", "Beto Mora", "Cris Solís", "Dora Ñandú", "Eli Vargas"],
  empresa: ["ACME", "Beta S.A.", "Cía. Ñandú", "Delta", "Épsilon y Asociados", "Fénix"],
  sitio: ["Brisas", "Planta Norte", "Bodega"],
};

function sumarDias(ymd: string, dias: number): string {
  const d = new Date(`${ymd}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() + dias);
  return d.toISOString().slice(0, 10);
}

function valorDe(campo: string, def: DefinicionCampo, azar: () => number): Fila[string] {
  if (azar() < 0.08) return null;
  switch (def.tipo) {
    case "texto": {
      const lista = TEXTOS[campo] ?? ["uno", "dos", "tres"];
      return lista[Math.floor(azar() * lista.length)];
    }
    case "enumeracion":
      return def.valores ? def.valores[Math.floor(azar() * def.valores.length)] : "X";
    case "fecha":
      return sumarDias(HOY, Math.floor(azar() * 500) - 250);
    case "numero":
      if (def.rango) return def.rango[0] + Math.floor(azar() * (def.rango[1] - def.rango[0] + 1));
      if (campo === "horas_adentro") return Math.round(azar() * 120) / 10;
      return Math.floor(azar() * 800) - 400;
    case "booleano":
      return azar() < 0.5;
  }
}

function filasDe(campos: Record<string, DefinicionCampo>, cuantas: number, semilla: number): Fila[] {
  const azar = generador(semilla);
  return Array.from({ length: cuantas }, () =>
    Object.fromEntries(Object.entries(campos).map(([campo, def]) => [campo, valorDe(campo, def, azar)])),
  );
}

/** Conjuntos de datos por los que pasa cada panel. */
function conjuntos(campos: Record<string, DefinicionCampo>): Record<string, Fila[]> {
  return {
    normales: filasDe(campos, 60, 7),
    "una fila": filasDe(campos, 1, 11),
    "sin filas": [],
    "todo vacío": Array.from({ length: 3 }, () => Object.fromEntries(Object.keys(campos).map((c) => [c, null]))),
  };
}

// --- Combinaciones -------------------------------------------------------------

type Documento = Record<string, unknown>;

function combinaciones(campos: Record<string, DefinicionCampo>): Documento[] {
  const nombres = Object.keys(campos);
  const de = (tipo: DefinicionCampo["tipo"]) => nombres.filter((n) => campos[n].tipo === tipo);
  const fechas = de("fecha");
  const numeros = de("numero");
  const categorias = nombres.filter((n) => ["texto", "enumeracion", "booleano"].includes(campos[n].tipo));
  const w: Documento[] = [];

  for (const medida of MEDIDAS) {
    for (const campo of [undefined, ...nombres]) {
      w.push({ tipo: "kpi", titulo: "KPI", medida, campo });
      w.push({ tipo: "kpi", titulo: "KPI", medida, campo, comparar: true, tendencia: true, tono: "advertencia" });
    }
  }
  for (const eje_x of nombres) {
    const agrupaciones = campos[eje_x].tipo === "fecha" ? ["dia", "semana", "mes"] : [undefined];
    for (const agrupar_fecha of agrupaciones) {
      for (const serie of [undefined, ...categorias.filter((c) => c !== eje_x)]) {
        w.push({ tipo: "barras", eje_x, serie, agrupar_fecha, medida: "contar" });
        w.push({ tipo: "linea", eje_x, serie, agrupar_fecha, medida: "contar", area: true });
      }
      w.push({ tipo: "barras", eje_x, agrupar_fecha, medida: "contar", orientacion: "horizontal", limite: 3 });
      w.push({ tipo: "barras", eje_x, agrupar_fecha, medida: "contar", orientacion: "vertical" });
      for (const campo of numeros) {
        for (const medida of ["sumar", "maximo", "contar_distintos"]) w.push({ tipo: "barras", eje_x, agrupar_fecha, medida, campo });
      }
    }
    w.push({ tipo: "torta", categoria: eje_x, medida: "contar" });
    w.push({ tipo: "torta", categoria: eje_x, medida: "contar", dona: true, limite: 2 });
    for (const columnas of nombres) w.push({ tipo: "calor", filas: eje_x, columnas, medida: "contar" });
    w.push({ tipo: "tabla", columnas: nombres.slice(0, 12), agrupar_por: eje_x, ...(fechas[0] ? { tendencia: fechas[0] } : {}) });
    w.push({ tipo: "tabla", columnas: [eje_x], orden: { campo: eje_x, dir: "desc" }, limite: 5 });
  }
  for (const fecha of fechas) {
    for (const meses of [undefined, 1, 12]) w.push({ tipo: "calendario", fecha, medida: "contar", meses });
    for (const campo of numeros) w.push({ tipo: "calendario", fecha, medida: "sumar", campo });
  }
  if (numeros.length) w.push({ tipo: "tabla", columnas: nombres, barras: numeros.slice(0, 4) });
  // Sin las claves `undefined` (el esquema es estricto, pero no las ve; mejor no mandarlas).
  return w.map((x) => Object.fromEntries(Object.entries(x).filter(([, v]) => v !== undefined)));
}

// --- Interacción ---------------------------------------------------------------

const errores: string[] = [];
/** El caso y el paso en curso, para nombrar el error que `PanelProtegido` informa por consola. */
let enCurso = "";
afterEach(cleanup);

function recorrer(nombre: string, lienzo: Lienzo, datos: Record<string, Fila[]>) {
  for (const [conjunto, filas] of Object.entries(datos)) {
    const caso = `${nombre} · ${conjunto}`;
    const paso = (texto: string) => (enCurso = `${caso} · ${texto}`);
    const ediciones: Lienzo[] = [];
    const vista = (l: Lienzo, f: Fila[]) => (
      <LienzoVista
        lienzo={l}
        filas={f}
        anteriores={f.slice(0, Math.floor(f.length / 2))}
        rango={{ desde: sumarDias(HOY, -13), hasta: HOY }}
        onEditar={(nuevo) => ediciones.push(nuevo)}
      />
    );
    try {
      paso("dibujar");
      const { container, rerender } = render(vista(lienzo, filas));
      // Teclado: entrar, recorrer hasta el final, elegir la última marca y la primera.
      paso("recorrer con el teclado");
      for (const grafico of container.querySelectorAll<SVGElement>('svg[role="img"][tabindex]')) {
        act(() => {
          fireEvent.focus(grafico);
          fireEvent.keyDown(grafico, { key: "ArrowRight" });
          fireEvent.keyDown(grafico, { key: "End" });
          fireEvent.keyDown(grafico, { key: "Enter" });
          fireEvent.keyDown(grafico, { key: "Home" });
          fireEvent.keyDown(grafico, { key: "Enter" });
          fireEvent.keyDown(grafico, { key: "End" });
        });
      }
      // Lo que la persona ve al filtrar: el documento nuevo, con menos datos, sin perder el resaltado.
      for (const editado of ediciones.slice(0, 2)) {
        const r = validarLienzo(editado);
        if (!r.ok) {
          errores.push(`${caso}: el filtro de un clic no pasa el esquema (${r.errores.join("; ")})`);
          continue;
        }
        paso(`filtrar (${JSON.stringify(r.lienzo.filtros)}) y quedar con menos datos`);
        rerender(vista(r.lienzo, filas.slice(0, 2)));
        rerender(vista(r.lienzo, filas.slice(0, 1)));
      }
      paso("quedar sin datos");
      rerender(vista(lienzo, []));
      paso("volver a los datos");
      rerender(vista(lienzo, filas));
      // Ver como tabla y volver.
      paso("ver como tabla");
      for (const boton of container.querySelectorAll<HTMLButtonElement>("button.lienzo-alternar")) {
        if (boton.getAttribute("aria-label")?.startsWith("Ver ")) {
          fireEvent.click(boton);
          fireEvent.click(boton);
        }
      }
    } catch (e) {
      errores.push(`${enCurso}: ${e instanceof Error ? e.message : String(e)}`);
    }
    cleanup();
  }
}

// --- Pruebas -------------------------------------------------------------------

function probarOrigen(
  titulo: string,
  fuente: Fuente | "consulta",
  campos: Record<string, DefinicionCampo>,
  minimo: number,
  columnas?: ColumnaConsulta[],
) {
  it(`${titulo}: cada combinación válida se dibuja, se recorre y se filtra sin romperse`, () => {
    // Cada panel que se rompe pasa por `PanelProtegido`, que lo informa por consola: ahí se anota.
    const silencio = vi.spyOn(console, "error").mockImplementation((primero: unknown, error?: unknown) => {
      if (typeof primero === "string" && primero.startsWith("Error al dibujar un panel")) {
        errores.push(`${enCurso}: ${error instanceof Error ? error.message : String(error)}`);
      }
    });
    const advertencias = vi.spyOn(console, "warn").mockImplementation(() => {});
    errores.length = 0;
    const datos = conjuntos(campos);
    let validas = 0;
    for (const widget of combinaciones(campos)) {
      const documento = { fuente, filtros: [], widgets: [widget], ...(columnas ? { consulta: { sql: "select 1", columnas } } : {}) };
      const r = validarLienzo(documento);
      if (!r.ok) continue;
      validas++;
      recorrer(JSON.stringify(widget), r.lienzo, datos);
    }
    silencio.mockRestore();
    advertencias.mockRestore();
    // Que el recorrido de verdad cubra algo (si el esquema cambia y rechaza todo, que se note).
    console.info(`${titulo}: ${validas} combinaciones`);
    expect(validas).toBeGreaterThan(minimo);
    expect(errores, errores.slice(0, 20).join("\n")).toEqual([]);
  }, 600_000);
}

describe("recorrido del lienzo", () => {
  for (const fuente of Object.keys(CATALOGO) as Fuente[]) {
    const campos: Record<string, DefinicionCampo> = CATALOGO[fuente].campos;
    probarOrigen(fuente, fuente, campos, 150);
  }
  const columnas: ColumnaConsulta[] = [
    { nombre: "empresa", tipo: "texto", etiqueta: "Empresa" },
    { nombre: "cantidad", tipo: "numero" },
    { nombre: "dia", tipo: "fecha", etiqueta: "Día" },
    { nombre: "activo", tipo: "booleano" },
  ];
  const origen = { columnas };
  probarOrigen(
    "consulta del asistente",
    "consulta",
    Object.fromEntries(columnas.map((c) => [c.nombre, definicionCampo(origen, c.nombre) as DefinicionCampo])),
    60,
    columnas,
  );
});
