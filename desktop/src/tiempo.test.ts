import { describe, expect, it } from "vitest";
import {
  fechaHaceMeses,
  fechaLocalYMD,
  interpretarFechaDDMMAAAA,
  mascaraFechaDDMMAAAA,
  textoFechaDDMMAAAA,
  textoFechaDDMMYYYY,
  textoHora,
} from "./tiempo";

// Se arma la fecha con getters LOCALES directos (no parseando un string UTC
// fijo), y se compara contra lo que el propio runner calcularía para esa
// misma fecha local — así el test no depende de en qué huso horario corre
// CI, pero sigue agarrando un bug real de padding/orden de campos.
describe("fechaLocalYMD / textoHora", () => {
  const fecha = new Date(2027, 2, 8, 14, 5); // 8 de marzo de 2027, 14:05 local
  const iso = fecha.toISOString();

  it("fechaLocalYMD arma año-mes-día con padding, en hora local", () => {
    const esperado = `${fecha.getFullYear()}-${String(fecha.getMonth() + 1).padStart(2, "0")}-${String(
      fecha.getDate(),
    ).padStart(2, "0")}`;
    expect(fechaLocalYMD(iso)).toBe(esperado);
  });

  it("textoHora es de 24 horas, con padding", () => {
    const esperado = fecha.toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
    expect(textoHora(iso)).toBe(esperado);
  });
});

describe("textoFechaDDMMYYYY", () => {
  it("reordena YYYY-MM-DD a DD/MM/YYYY", () => {
    expect(textoFechaDDMMYYYY("2027-03-08")).toBe("08/03/2027");
  });

  it("con padding de un solo dígito (mes/día de un dígito ya vienen paddeados)", () => {
    expect(textoFechaDDMMYYYY("2027-01-05")).toBe("05/01/2027");
  });
});

describe("fechaHaceMeses", () => {
  it("resta meses dentro del mismo año", () => {
    expect(fechaHaceMeses(6, new Date(2026, 7, 15))).toBe("2026-02-15");
  });

  it("cruza el límite de año", () => {
    expect(fechaHaceMeses(6, new Date(2026, 2, 15))).toBe("2025-09-15");
  });

  it("cae en un mes más corto: se ajusta (clamp) a su último día, no rebalsa al siguiente", () => {
    // 31 de marzo menos 1 mes "debería" ser 31 de febrero, que no existe —
    // en vez de dejar que rebalse al 3 de marzo (comportamiento crudo de
    // `Date`), se ajusta al último día real de febrero.
    expect(fechaHaceMeses(1, new Date(2026, 2, 31))).toBe("2026-02-28");
  });

  it("año bisiesto: el último día de febrero es el 29, no el 28", () => {
    expect(fechaHaceMeses(1, new Date(2028, 2, 31))).toBe("2028-02-29");
  });
});

describe("mascaraFechaDDMMAAAA", () => {
  it("pone las barras mientras se escribe", () => {
    expect(mascaraFechaDDMMAAAA("2")).toBe("2");
    expect(mascaraFechaDDMMAAAA("230")).toBe("23/0");
    expect(mascaraFechaDDMMAAAA("23092")).toBe("23/09/2");
    expect(mascaraFechaDDMMAAAA("23092026")).toBe("23/09/2026");
  });

  it("ignora lo que no es dígito y corta en 8 dígitos", () => {
    expect(mascaraFechaDDMMAAAA("23/09/2026")).toBe("23/09/2026");
    expect(mascaraFechaDDMMAAAA("23-09-20261")).toBe("23/09/2026");
    expect(mascaraFechaDDMMAAAA("ab")).toBe("");
  });
});

describe("interpretarFechaDDMMAAAA / textoFechaDDMMAAAA", () => {
  it("interpreta una fecha completa en hora local", () => {
    const fecha = interpretarFechaDDMMAAAA("23/09/2026");
    expect(fecha).not.toBeNull();
    expect(fecha && textoFechaDDMMAAAA(fecha)).toBe("23/09/2026");
    expect(fecha?.getHours()).toBe(0);
  });

  it("rechaza incompletas o inexistentes", () => {
    expect(interpretarFechaDDMMAAAA("23/09/20")).toBeNull();
    expect(interpretarFechaDDMMAAAA("31/02/2026")).toBeNull();
    expect(interpretarFechaDDMMAAAA("00/01/2026")).toBeNull();
  });
});
