import { describe, expect, it } from "vitest";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora, textoMedio } from "./Historial";

describe("textoMedio", () => {
  it("Vehiculo -> Vehículo, cualquier otro -> Caminando", () => {
    expect(textoMedio("Vehiculo")).toBe("Vehículo");
    expect(textoMedio("Caminando")).toBe("Caminando");
  });
});

// Agnósticas al huso horario del runner — ver el mismo criterio en
// Activos.test.ts.
describe("fechaLocalYMD / textoHora", () => {
  const fecha = new Date(2027, 2, 8, 14, 5);
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
});
