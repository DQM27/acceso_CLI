import { describe, expect, it } from "vitest";
import { escaparHtml, filaDinamica, permanenciaDeCelda } from "./Analisis.logica";

describe("filaDinamica", () => {
  it("deja el día sin zona horaria y le pone su día de la semana", () => {
    const fila = filaDinamica(
      {
        dia: "2026-03-01",
        unidad: "Brisas",
        tipo_persona: "PROVEEDOR",
        tipo_ingreso: "—",
        medio: "VEHÍCULO",
        ingresos: 4,
        con_salida: 3,
        minutos_adentro: 150,
      },
      "Proveedores",
    );

    // El 1 de marzo de 2026 es domingo.
    expect(fila).toEqual({
      dia: "2026-03-01T00:00:00",
      dia_semana: "Domingo",
      unidad: "Brisas",
      tipo_persona: "Proveedores",
      tipo_ingreso: "—",
      medio: "VEHÍCULO",
      ingresos: 4,
      con_salida: 3,
      minutos_adentro: 150,
      horas_adentro: 3,
      permanencia: 0,
    });
  });
});

describe("permanenciaDeCelda", () => {
  it("pondera por salidas en vez de promediar los promedios", () => {
    // Día 1: 10 salidas de 60 min; día 2: 1 salida de 600 min.
    // Promedio de promedios = 330; ponderado = 1200 / 11 ≈ 109.
    expect(
      permanenciaDeCelda([
        { minutos_adentro: 600, con_salida: 10 },
        { minutos_adentro: 600, con_salida: 1 },
      ]),
    ).toBe(109);
  });

  it("deja la celda vacía si nadie salió", () => {
    expect(permanenciaDeCelda([{ minutos_adentro: 0, con_salida: 0 }])).toBeUndefined();
    expect(permanenciaDeCelda([])).toBeUndefined();
  });
});

describe("escaparHtml", () => {
  it("neutraliza el marcado en textos escritos por operadores", () => {
    expect(escaparHtml(`<img src=x onerror="alert(1)">&'`)).toBe(
      "&#60;img src=x onerror=&#34;alert(1)&#34;&#62;&#38;&#39;",
    );
    expect(escaparHtml("CONSTRUCTORA SÁNCHEZ")).toBe("CONSTRUCTORA SÁNCHEZ");
  });
});
