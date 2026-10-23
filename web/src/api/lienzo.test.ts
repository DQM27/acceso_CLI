import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Lienzo } from "../lienzo/esquema";
import { consultarLienzo, contarConsultasDeHoy, hoyCostaRica } from "./lienzo";

const mocks = vi.hoisted(() => ({ rpc: vi.fn(), invoke: vi.fn(), from: vi.fn() }));
vi.mock("../lib/supabase", () => ({ supabase: { rpc: mocks.rpc, functions: { invoke: mocks.invoke }, from: mocks.from } }));

beforeEach(() => {
  vi.clearAllMocks();
});

const rango: Lienzo["filtros"] = [{ campo: "dia", op: "entre", valor: ["2026-10-09", "2026-10-22"] }];

describe("consultarLienzo", () => {
  it("contratistas: llama a su función con los filtros traducidos", async () => {
    mocks.rpc.mockResolvedValue({ data: [{ nombre: "Ana" }], error: null });
    const r = await consultarLienzo({
      fuente: "contratistas",
      filtros: [{ campo: "empresa", op: "igual", valor: "ACME" }],
      widgets: [{ tipo: "kpi", titulo: "t", medida: "contar" }],
    });
    expect(mocks.rpc).toHaveBeenCalledWith("lienzo_contratistas", { p_empresas: ["ACME"] });
    expect(r).toEqual({ filas: [{ nombre: "Ana" }] });
  });

  it("ingresos sin comparar: sólo el rango pedido", async () => {
    mocks.rpc.mockResolvedValue({ data: [], error: null });
    const r = await consultarLienzo({ fuente: "ingresos", filtros: rango, widgets: [{ tipo: "kpi", titulo: "t", medida: "contar" }] });
    expect(mocks.rpc).toHaveBeenCalledWith("lienzo_ingresos", { p_desde: "2026-10-09", p_hasta: "2026-10-22" });
    expect(r.rango).toEqual({ desde: "2026-10-09", hasta: "2026-10-22" });
    expect(r.anteriores).toBeUndefined();
  });

  it("ingresos con un KPI que compara: pide también el período anterior y lo separa", async () => {
    mocks.rpc.mockResolvedValue({
      data: [{ dia: "2026-10-20" }, { dia: "2026-10-01" }, { dia: "2026-09-25" }],
      error: null,
    });
    const r = await consultarLienzo({
      fuente: "ingresos",
      filtros: rango,
      widgets: [{ tipo: "kpi", titulo: "t", medida: "contar", comparar: true }],
    });
    // 14 días pedidos → los 14 anteriores empiezan el 25/09.
    expect(mocks.rpc).toHaveBeenCalledWith("lienzo_ingresos", { p_desde: "2026-09-25", p_hasta: "2026-10-22" });
    expect(r.filas).toEqual([{ dia: "2026-10-20" }]);
    expect(r.anteriores).toEqual([{ dia: "2026-10-01" }, { dia: "2026-09-25" }]);
  });

  it("si el doble del rango pasa de 92 días, no compara (la función SQL no lo permite)", async () => {
    mocks.rpc.mockResolvedValue({ data: [], error: null });
    const r = await consultarLienzo({
      fuente: "ingresos",
      filtros: [{ campo: "dia", op: "entre", valor: ["2026-08-01", "2026-10-22"] }],
      widgets: [{ tipo: "kpi", titulo: "t", medida: "contar", comparar: true }],
    });
    expect(mocks.rpc.mock.calls[0][1].p_desde).toBe("2026-08-01");
    expect(r.anteriores).toBeUndefined();
  });

  it("un error de la función se avisa con su mensaje", async () => {
    mocks.rpc.mockResolvedValue({ data: null, error: { message: "Sólo los administradores del panel pueden usar el lienzo" } });
    await expect(
      consultarLienzo({ fuente: "contratistas", filtros: [], widgets: [{ tipo: "kpi", titulo: "t", medida: "contar" }] }),
    ).rejects.toThrow("administradores del panel");
  });

  describe('fuente "consulta"', () => {
    const lienzo = (filtros: Lienzo["filtros"] = []): Lienzo => ({
      fuente: "consulta",
      consulta: {
        sql: "select nombre, (hora_entrada at time zone 'America/Costa_Rica')::date as dia, datos from ia.x",
        columnas: [
          { nombre: "nombre", tipo: "texto" },
          { nombre: "dia", tipo: "fecha" },
          { nombre: "datos", tipo: "texto" },
        ],
      },
      filtros,
      widgets: [{ tipo: "tabla", columnas: ["nombre", "dia"] }],
    });

    it("la corre con lienzo-consulta (no con el modelo), deja las fechas en el día y aplica los filtros", async () => {
      mocks.invoke.mockResolvedValue({
        data: {
          columnas: ["nombre", "dia", "datos"],
          filas: [
            { nombre: "Ana", dia: "2026-10-20T00:00:00", datos: { a: 1 } },
            { nombre: "Beto", dia: "2026-10-21", datos: null },
          ],
          truncado: false,
        },
        error: null,
      });
      const r = await consultarLienzo(lienzo([{ campo: "nombre", op: "contiene", valor: "an" }]));
      expect(mocks.invoke).toHaveBeenCalledWith("lienzo-consulta", { body: { sql: lienzo().consulta?.sql } });
      expect(mocks.rpc).not.toHaveBeenCalled();
      expect(r).toEqual({ filas: [{ nombre: "Ana", dia: "2026-10-20", datos: '{"a":1}' }], truncado: false });
    });

    it("si la consulta no devuelve las columnas declaradas, avisa en vez de dibujar a medias", async () => {
      mocks.invoke.mockResolvedValue({ data: { columnas: ["nombre"], filas: [{ nombre: "Ana" }], truncado: false }, error: null });
      await expect(consultarLienzo(lienzo())).rejects.toThrow("no devolvió: dia, datos");
    });
  });
});

describe("contarConsultasDeHoy", () => {
  it("cuenta desde el inicio del día en Costa Rica y sin las frenadas por el tope", async () => {
    const consulta = { select: vi.fn(), gte: vi.fn(), neq: vi.fn() };
    consulta.select.mockReturnValue(consulta);
    consulta.gte.mockReturnValue(consulta);
    consulta.neq.mockResolvedValue({ count: 7, error: null });
    mocks.from.mockReturnValue(consulta);

    // 03:00 UTC del 23 = todavía 22 en Costa Rica.
    expect(await contarConsultasDeHoy(new Date("2026-10-23T03:00:00Z"))).toBe(7);
    expect(mocks.from).toHaveBeenCalledWith("lienzo_ia_uso");
    expect(consulta.select).toHaveBeenCalledWith("id", { count: "exact", head: true });
    expect(consulta.gte).toHaveBeenCalledWith("creado_en", "2026-10-22T06:00:00.000Z");
    expect(consulta.neq).toHaveBeenCalledWith("resultado", "tope");
  });
});

describe("hoyCostaRica", () => {
  it("usa el día de Costa Rica (UTC-6), no el de UTC", () => {
    expect(hoyCostaRica(new Date("2026-10-23T03:00:00Z"))).toBe("2026-10-22");
    expect(hoyCostaRica(new Date("2026-10-23T07:00:00Z"))).toBe("2026-10-23");
  });
});
