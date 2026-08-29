import { describe, expect, it } from "vitest";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "./tiempo";

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
