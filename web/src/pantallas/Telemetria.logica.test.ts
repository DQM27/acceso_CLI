import { describe, expect, it } from "vitest";
import { GRUPOS_TELEMETRIA } from "../api/telemetria";
import {
  contarPorEstado,
  estadoEnvioTelemetria,
  textoMs,
  textoPorcentajeErrores,
  esLoDeSiempre,
  estadoEstudio,
  gruposVigentes,
  ordenVencida,
  ordenarGrupos,
  textoGrupos,
  textoVence,
  venceEnDesde,
} from "./Telemetria.logica";

const AHORA = new Date("2026-10-20T12:00:00Z");

describe("ordenVencida", () => {
  it("sin vencimiento nunca vence", () => {
    expect(ordenVencida({ grupos: ["fluidez"], vence_en: null }, AHORA)).toBe(false);
  });

  it("vence justo en la hora indicada", () => {
    expect(ordenVencida({ grupos: [], vence_en: "2026-10-20T12:00:00Z" }, AHORA)).toBe(true);
    expect(ordenVencida({ grupos: [], vence_en: "2026-10-20T12:00:01Z" }, AHORA)).toBe(false);
  });
});

describe("gruposVigentes", () => {
  it("sin orden mide lo de siempre", () => {
    expect(gruposVigentes(undefined, AHORA)).toEqual(["rendimiento"]);
  });

  it("con orden vigente mide lo de la orden, aunque sea sólo base", () => {
    expect(gruposVigentes({ grupos: ["fluidez", "recursos"], vence_en: null }, AHORA)).toEqual([
      "fluidez",
      "recursos",
    ]);
    expect(gruposVigentes({ grupos: [], vence_en: null }, AHORA)).toEqual([]);
  });

  it("con la orden vencida vuelve a lo de siempre", () => {
    expect(gruposVigentes({ grupos: ["depuracion"], vence_en: "2026-10-19T12:00:00Z" }, AHORA)).toEqual([
      "rendimiento",
    ]);
  });
});

describe("textoGrupos", () => {
  it("nombra base siempre", () => {
    expect(textoGrupos([])).toBe("Sólo base");
    expect(textoGrupos(["rendimiento", "fluidez"])).toBe("base + rendimiento, fluidez");
  });
});

describe("esLoDeSiempre", () => {
  it("sólo rendimiento es lo de siempre", () => {
    expect(esLoDeSiempre(["rendimiento"])).toBe(true);
    expect(esLoDeSiempre([])).toBe(false);
    expect(esLoDeSiempre(["rendimiento", "fluidez"])).toBe(false);
  });
});

describe("venceEnDesde", () => {
  it("suma las horas a ahora", () => {
    expect(venceEnDesde(24, AHORA)).toBe("2026-10-21T12:00:00.000Z");
  });

  it("null es sin vencimiento", () => {
    expect(venceEnDesde(null, AHORA)).toBeNull();
  });
});

describe("estadoEstudio", () => {
  it("sin orden, vencida o pidiendo lo de siempre es lo de siempre", () => {
    expect(estadoEstudio(undefined, AHORA)).toBe("siempre");
    expect(estadoEstudio({ grupos: ["fluidez"], vence_en: "2026-10-19T12:00:00Z" }, AHORA)).toBe("siempre");
    expect(estadoEstudio({ grupos: ["rendimiento"], vence_en: null }, AHORA)).toBe("siempre");
  });

  it("más o menos estudios que lo de siempre es una orden", () => {
    expect(estadoEstudio({ grupos: ["rendimiento", "fluidez"], vence_en: null }, AHORA)).toBe("con_orden");
    expect(estadoEstudio({ grupos: [], vence_en: "2026-10-21T12:00:00Z" }, AHORA)).toBe("con_orden");
  });
});

describe("contarPorEstado", () => {
  it("cuenta cada estado, con cero si no hay", () => {
    expect(contarPorEstado(["siempre", "con_orden", "siempre"])).toEqual({ con_orden: 1, siempre: 2 });
    expect(contarPorEstado([])).toEqual({ con_orden: 0, siempre: 0 });
  });
});

describe("textoVence", () => {
  const fecha = (iso: string) => `[${iso}]`;

  it("sin orden vigente no hay vencimiento que mostrar", () => {
    expect(textoVence(undefined, AHORA, fecha)).toBe("—");
    expect(textoVence({ grupos: [], vence_en: "2026-10-20T11:00:00Z" }, AHORA, fecha)).toBe("—");
  });

  it("una orden sin vencimiento lo dice", () => {
    expect(textoVence({ grupos: [], vence_en: null }, AHORA, fecha)).toBe("Sin vencimiento");
  });

  it("dice cuánto falta: menos de 1 h, en horas hasta 2 días y luego en días", () => {
    const vence = (iso: string) => textoVence({ grupos: [], vence_en: iso }, AHORA, fecha);
    expect(vence("2026-10-20T12:30:00Z")).toBe("[2026-10-20T12:30:00Z] (en menos de 1 h)");
    expect(vence("2026-10-21T17:00:00Z")).toBe("[2026-10-21T17:00:00Z] (en 29 h)");
    expect(vence("2026-10-27T12:00:00Z")).toBe("[2026-10-27T12:00:00Z] (en 7 d)");
  });
});

describe("ordenarGrupos", () => {
  it("devuelve los marcados en el orden fijo de la lista", () => {
    expect(ordenarGrupos(new Set(["escaneo", "rendimiento", "fluidez"] as const), GRUPOS_TELEMETRIA)).toEqual([
      "rendimiento",
      "fluidez",
      "escaneo",
    ]);
  });
});

describe("estadoEnvioTelemetria", () => {
  const ahora = new Date("2026-11-01T12:00:00-06:00");
  const hace = (minutos: number) => new Date(ahora.getTime() - minutos * 60_000).toISOString();

  it("al día hasta 20 minutos, atrasado hasta 2 horas, después sin señal", () => {
    expect(estadoEnvioTelemetria(hace(1), ahora)).toBe("al_dia");
    expect(estadoEnvioTelemetria(hace(20), ahora)).toBe("al_dia");
    expect(estadoEnvioTelemetria(hace(21), ahora)).toBe("atrasado");
    expect(estadoEnvioTelemetria(hace(120), ahora)).toBe("atrasado");
    expect(estadoEnvioTelemetria(hace(121), ahora)).toBe("sin_senal");
  });

  it("sin ningún lote es sin señal", () => {
    expect(estadoEnvioTelemetria(null, ahora)).toBe("sin_senal");
  });
});

describe("textos de rendimiento", () => {
  it("porcentaje de errores con un decimal y guion sin llamadas", () => {
    expect(textoPorcentajeErrores(12, 1)).toBe("8,3 %");
    expect(textoPorcentajeErrores(10, 0)).toBe("0 %");
    expect(textoPorcentajeErrores(0, 0)).toBe("—");
  });

  it("milisegundos con un decimal por debajo de 100 y sin decimales desde 100", () => {
    expect(textoMs(38.5)).toBe("38,5 ms");
    expect(textoMs(276.4)).toBe("276 ms");
    expect(textoMs(null)).toBe("—");
  });
});
