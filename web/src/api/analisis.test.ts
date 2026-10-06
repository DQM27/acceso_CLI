import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  diasDelRango,
  obtenerResumenMovimientos,
  problemaDelRango,
  textoPermanencia,
} from "./analisis";

const mocks = vi.hoisted(() => ({ rpc: vi.fn() }));
vi.mock("../lib/supabase", () => ({ supabase: { rpc: mocks.rpc } }));

beforeEach(() => {
  vi.clearAllMocks();
});

const RESUMEN_VACIO = {
  diario: [],
  por_hora: [],
  empresas: [],
  total: { ingresos: 0, con_salida: 0, minutos_adentro: 0, personas: 0 },
};

describe("obtenerResumenMovimientos", () => {
  it("pide el período completo en días de Costa Rica, con el 'hasta' inclusive", async () => {
    mocks.rpc.mockResolvedValue({ data: RESUMEN_VACIO, error: null });

    await obtenerResumenMovimientos({ desde: "2026-03-01", hasta: "2026-03-31", sitioIds: ["s1"] }, "2026-09-30");

    expect(mocks.rpc).toHaveBeenCalledWith("panel_resumen_movimientos", {
      p_desde: "2026-03-01T00:00:00-06:00",
      p_hasta: "2026-04-01T00:00:00-06:00",
      p_sitio_ids: ["s1"],
    });
  });

  it("sin 'hasta' resume hasta el final de hoy, y sin unidades no filtra", async () => {
    mocks.rpc.mockResolvedValue({ data: RESUMEN_VACIO, error: null });

    await obtenerResumenMovimientos({ desde: "2026-09-01" }, "2026-09-30");

    expect(mocks.rpc).toHaveBeenCalledWith("panel_resumen_movimientos", {
      p_desde: "2026-09-01T00:00:00-06:00",
      p_hasta: "2026-10-01T00:00:00-06:00",
      p_sitio_ids: null,
    });
  });

  it("propaga el error de la base con su mensaje", async () => {
    mocks.rpc.mockResolvedValue({ data: null, error: { message: "El resumen admite hasta un año" } });

    await expect(obtenerResumenMovimientos({ desde: "2026-09-01" }, "2026-09-30")).rejects.toThrow(
      "El resumen admite hasta un año",
    );
  });

  it("rechaza una respuesta con forma inesperada en vez de fallar más adelante", async () => {
    mocks.rpc.mockResolvedValue({ data: { diario: [] }, error: null });

    await expect(obtenerResumenMovimientos({ desde: "2026-09-01" }, "2026-09-30")).rejects.toThrow();
  });
});

describe("problemaDelRango", () => {
  it("acepta de un día hasta un año, contando los dos extremos", () => {
    expect(diasDelRango("2026-03-02", "2026-03-02")).toBe(1);
    expect(problemaDelRango("2026-03-02", "2026-03-02")).toBeNull();
    // 2024 es bisiesto: del 1 de enero al 31 de diciembre son 366 días.
    expect(problemaDelRango("2024-01-01", "2024-12-31")).toBeNull();
  });

  it("rechaza rangos invertidos, de más de un año o sin inicio", () => {
    expect(problemaDelRango("2026-03-02", "2026-03-01")).toMatch(/anterior/);
    expect(problemaDelRango("2025-01-01", "2026-01-02")).toMatch(/un año/);
    expect(problemaDelRango("", "2026-01-02")).toMatch(/desde/);
  });
});

describe("textoPermanencia", () => {
  it("promedia sólo sobre quienes ya salieron", () => {
    expect(textoPermanencia(90, 2)).toBe("45 min");
    expect(textoPermanencia(240, 2)).toBe("2 h");
    expect(textoPermanencia(275, 2)).toBe("2 h 18 min");
  });

  it("sin salidas no inventa un promedio", () => {
    expect(textoPermanencia(0, 0)).toBe("—");
  });
});
