import { describe, expect, it } from "vitest";
import { sanearSoloDigitos, sanearSoloLetras } from "./validacion";

// Mismos casos que desktop/src/validacion.test.ts para las dos funciones
// que sí existen acá (ver el doc-comment de validacion.ts -- esta copia no
// trae cedulaSchema/nombreSchema, sólo el saneamiento en vivo).
describe("sanearSoloDigitos", () => {
  it("descarta todo lo que no sea dígito", () => {
    expect(sanearSoloDigitos("1-0847-0293")).toBe("108470293");
  });
});

describe("sanearSoloLetras", () => {
  it("descarta números y símbolos, conserva acentos/espacio/apóstrofe/guión", () => {
    expect(sanearSoloLetras("José O'Neill Pérez-Ruiz 2")).toBe("José O'Neill Pérez-Ruiz ");
  });
});
