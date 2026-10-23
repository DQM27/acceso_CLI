import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Lienzo } from "../lienzo/esquema";
import { consultarLienzo, hoyCostaRica } from "./lienzo";

const mocks = vi.hoisted(() => ({ rpc: vi.fn() }));
vi.mock("../lib/supabase", () => ({ supabase: { rpc: mocks.rpc } }));

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
});

describe("hoyCostaRica", () => {
  it("usa el día de Costa Rica (UTC-6), no el de UTC", () => {
    expect(hoyCostaRica(new Date("2026-10-23T03:00:00Z"))).toBe("2026-10-22");
    expect(hoyCostaRica(new Date("2026-10-23T07:00:00Z"))).toBe("2026-10-23");
  });
});
