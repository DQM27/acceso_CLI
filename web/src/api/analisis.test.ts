import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  diasDelRango,
  diasPorTipo,
  ingresosPorHora,
  obtenerResumenMovimientos,
  problemaDelRango,
  tendenciaDiaria,
  textoPermanencia,
} from "./analisis";
import type { ResumenDiario } from "./analisis";

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

function diario(sobrescribir: Partial<ResumenDiario>): ResumenDiario {
  return {
    dia: "2026-03-02",
    unidad: "Brisas",
    tipo_persona: "CONTRATISTA",
    tipo_ingreso: "IN HOUSE",
    medio: "CAMINANDO",
    ingresos: 1,
    con_salida: 1,
    minutos_adentro: 60,
    ...sobrescribir,
  };
}

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

describe("tendenciaDiaria", () => {
  it("separa contratistas de proveedores y rellena con cero los días sin movimientos", () => {
    const puntos = tendenciaDiaria(
      [
        diario({ dia: "2026-03-01", ingresos: 3 }),
        diario({ dia: "2026-03-01", unidad: "Otra", ingresos: 2 }),
        diario({ dia: "2026-03-03", tipo_persona: "PROVEEDOR", ingresos: 4 }),
      ],
      "2026-03-01",
      "2026-03-03",
    );

    expect(puntos).toEqual([
      { dia: "2026-03-01", contratistas: 5, proveedores: 0 },
      { dia: "2026-03-02", contratistas: 0, proveedores: 0 },
      { dia: "2026-03-03", contratistas: 0, proveedores: 4 },
    ]);
  });

  it("cruza fin de mes y de año sin saltarse días", () => {
    expect(tendenciaDiaria([], "2025-12-30", "2026-01-02").map((p) => p.dia)).toEqual([
      "2025-12-30",
      "2025-12-31",
      "2026-01-01",
      "2026-01-02",
    ]);
  });
});

describe("ingresosPorHora", () => {
  it("promedia por día de cada tipo, con las 24 horas siempre", () => {
    // Del lunes 2 al domingo 15 de marzo de 2026: 10 días entre semana y 4 de fin de semana.
    expect(diasPorTipo("2026-03-02", "2026-03-15")).toEqual({ entreSemana: 10, finDeSemana: 4 });

    const horas = ingresosPorHora(
      [
        { dia_semana: 1, hora: 7, ingresos: 10 },
        { dia_semana: 5, hora: 7, ingresos: 5 },
        { dia_semana: 6, hora: 7, ingresos: 2 },
        { dia_semana: 7, hora: 23, ingresos: 1 },
      ],
      "2026-03-02",
      "2026-03-15",
    );

    expect(horas).toHaveLength(24);
    expect(horas[7]).toEqual({ hora: 7, etiqueta: "07:00", entreSemana: 1.5, finDeSemana: 0.5 });
    expect(horas[23]).toEqual({ hora: 23, etiqueta: "23:00", entreSemana: 0, finDeSemana: 0.3 });
    expect(horas[0]).toEqual({ hora: 0, etiqueta: "00:00", entreSemana: 0, finDeSemana: 0 });
  });

  it("un período sin fin de semana no divide entre cero", () => {
    const horas = ingresosPorHora([{ dia_semana: 2, hora: 8, ingresos: 3 }], "2026-03-03", "2026-03-03");
    expect(horas[8]).toEqual({ hora: 8, etiqueta: "08:00", entreSemana: 3, finDeSemana: 0 });
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
