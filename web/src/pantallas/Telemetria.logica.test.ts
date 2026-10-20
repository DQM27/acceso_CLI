import { describe, expect, it } from "vitest";
import { GRUPOS_TELEMETRIA } from "../api/telemetria";
import {
  coincideBusqueda,
  esLoDeSiempre,
  gruposVigentes,
  ordenVencida,
  ordenarGrupos,
  textoGrupos,
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

describe("coincideBusqueda", () => {
  const FILA = ["APK debug placa", "Brisas - Oficinas", "Celular"];

  it("sin nada tecleado coincide todo", () => {
    expect(coincideBusqueda(FILA, "")).toBe(true);
    expect(coincideBusqueda(FILA, "   ")).toBe(true);
  });

  it("ignora tildes y mayúsculas, y busca en etiqueta, unidad y tipo", () => {
    expect(coincideBusqueda(FILA, "PLÁCA")).toBe(true);
    expect(coincideBusqueda(FILA, "oficinas")).toBe(true);
    expect(coincideBusqueda(FILA, "celular")).toBe(true);
  });

  it("todas las palabras tienen que aparecer", () => {
    expect(coincideBusqueda(FILA, "brisas debug")).toBe(true);
    expect(coincideBusqueda(FILA, "brisas emulador")).toBe(false);
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
