import { describe, expect, it } from "vitest";
import { PRESETS } from "./SelectorRangoFecha";

function rangoDe(etiqueta: string, hoy: Date) {
  const preset = PRESETS.find((p) => p.etiqueta === etiqueta);
  if (!preset) throw new Error(`No existe el preset "${etiqueta}"`);
  return preset.calcular(hoy);
}

describe("PRESETS de SelectorRangoFecha", () => {
  // Miércoles 12 de agosto de 2026 — día de semana "del medio" a propósito,
  // para que "esta semana"/"semana pasada" no coincidan por casualidad con
  // el propio hoy.
  const hoy = new Date(2026, 7, 12);

  it("Hoy y Ayer son un solo día", () => {
    expect(rangoDe("Hoy", hoy)).toEqual({ desde: "2026-08-12", hasta: "2026-08-12" });
    expect(rangoDe("Ayer", hoy)).toEqual({ desde: "2026-08-11", hasta: "2026-08-11" });
  });

  it("Esta semana arranca el lunes, no el domingo", () => {
    expect(rangoDe("Esta semana", hoy)).toEqual({ desde: "2026-08-10", hasta: "2026-08-12" });
  });

  it("Semana pasada es la semana completa anterior (lunes a domingo)", () => {
    expect(rangoDe("Semana pasada", hoy)).toEqual({ desde: "2026-08-03", hasta: "2026-08-09" });
  });

  it("Este mes va del día 1 hasta hoy", () => {
    expect(rangoDe("Este mes", hoy)).toEqual({ desde: "2026-08-01", hasta: "2026-08-12" });
  });

  it("Mes pasado es el mes calendario completo anterior", () => {
    expect(rangoDe("Mes pasado", hoy)).toEqual({ desde: "2026-07-01", hasta: "2026-07-31" });
  });

  it("Mes pasado cruza el límite de año correctamente (enero → diciembre del año anterior)", () => {
    const eneroDe2026 = new Date(2026, 0, 15);
    expect(rangoDe("Mes pasado", eneroDe2026)).toEqual({
      desde: "2025-12-01",
      hasta: "2025-12-31",
    });
  });

  it("Últimos 7 días incluye hoy (6 días atrás + hoy = 7)", () => {
    expect(rangoDe("Últimos 7 días", hoy)).toEqual({ desde: "2026-08-06", hasta: "2026-08-12" });
  });

  it("Últimos 30 días incluye hoy (29 días atrás + hoy = 30)", () => {
    expect(rangoDe("Últimos 30 días", hoy)).toEqual({ desde: "2026-07-14", hasta: "2026-08-12" });
  });
});
