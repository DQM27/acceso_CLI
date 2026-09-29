import { describe, expect, it } from "vitest";
import { codigoCompleto, formatearCodigo, normalizarCodigo } from "./CodigoVinculacion.logica";

describe("código de vinculación", () => {
  it("normaliza a mayúsculas y sin separadores, como lo compara el servidor", () => {
    expect(normalizarCodigo(" k7qm-r4xt 2p ")).toBe("K7QMR4XT2P");
  });

  it("agrupa 4-4-2 mientras se escribe, igual que lo muestra el panel", () => {
    expect(formatearCodigo("k7")).toBe("K7");
    expect(formatearCodigo("k7qmr")).toBe("K7QM-R");
    expect(formatearCodigo("K7QMR4XT2P")).toBe("K7QM-R4XT-2P");
  });

  it("no deja escribir más allá del largo del código", () => {
    expect(formatearCodigo("K7QMR4XT2PZZZ")).toBe("K7QM-R4XT-2P");
  });

  it("sólo está completo con los 10 caracteres", () => {
    expect(codigoCompleto("K7QM-R4XT")).toBe(false);
    expect(codigoCompleto("K7QM-R4XT-2P")).toBe(true);
  });
});
