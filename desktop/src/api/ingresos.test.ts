import { describe, expect, it } from "vitest";
import { gafetesDe, sanearGafetes, textoMedio, textoMedioConPlaca } from "./ingresos";

describe("textoMedio", () => {
  it("Vehiculo -> Vehículo, cualquier otro -> Caminando", () => {
    expect(textoMedio("Vehiculo")).toBe("VEHÍCULO");
    expect(textoMedio("Caminando")).toBe("CAMINANDO");
  });
});

describe("textoMedioConPlaca", () => {
  it("Vehiculo con placa muestra la placa", () => {
    expect(textoMedioConPlaca("Vehiculo", "ABC123")).toBe("ABC123");
  });

  it("Vehiculo sin placa (dato viejo pre-migración) cae a 'Vehículo'", () => {
    expect(textoMedioConPlaca("Vehiculo", null)).toBe("VEHÍCULO");
    expect(textoMedioConPlaca("Vehiculo", "   ")).toBe("VEHÍCULO");
  });

  it("Caminando siempre muestra 'Caminando', tenga placa o no", () => {
    expect(textoMedioConPlaca("Caminando", null)).toBe("CAMINANDO");
    expect(textoMedioConPlaca("Caminando", "ABC123")).toBe("CAMINANDO");
  });
});

describe("sanearGafetes", () => {
  it("conserva dígitos, comas y espacios", () => {
    expect(sanearGafetes("2, 25, 85")).toBe("2, 25, 85");
  });

  it("descarta cualquier otro carácter", () => {
    expect(sanearGafetes("2a, 25!, 85#")).toBe("2, 25, 85");
  });

  it("trunca a 60 caracteres", () => {
    const entrada = "1".repeat(100);
    expect(sanearGafetes(entrada)).toHaveLength(60);
  });
});

describe("gafetesDe", () => {
  it("parsea una lista separada por comas", () => {
    expect(gafetesDe("2, 25, 85")).toEqual([2, 25, 85]);
  });

  it("ignora tokens vacíos (comas de más, espacios)", () => {
    expect(gafetesDe("2,, 25,  ,85,")).toEqual([2, 25, 85]);
  });

  it("descarta tokens no enteros", () => {
    expect(gafetesDe("2, abc, 2.5, 85")).toEqual([2, 85]);
  });

  it("texto vacío da lista vacía", () => {
    expect(gafetesDe("")).toEqual([]);
  });
});
