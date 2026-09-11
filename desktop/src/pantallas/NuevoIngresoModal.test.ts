import { describe, expect, it } from "vitest";
import { validarGafete } from "./NuevoIngresoModal";

describe("validarGafete", () => {
  it("sin gafete requerido, siempre válido con numero null", () => {
    expect(validarGafete("", false)).toEqual({ valido: true, numero: null });
    expect(validarGafete("basura", false)).toEqual({ valido: true, numero: null });
  });

  it("gafete requerido y vacío", () => {
    expect(validarGafete("", true)).toEqual({ valido: false, mensaje: "El gafete es requerido" });
    expect(validarGafete("   ", true)).toEqual({ valido: false, mensaje: "El gafete es requerido" });
  });

  it("gafete requerido y no numérico", () => {
    expect(validarGafete("abc", true)).toEqual({
      valido: false,
      mensaje: "Ingrese un número de gafete válido",
    });
  });

  it("gafete requerido y válido", () => {
    expect(validarGafete("12", true)).toEqual({ valido: true, numero: 12 });
    expect(validarGafete("  7  ", true)).toEqual({ valido: true, numero: 7 });
  });

  it("acepta números con texto arrastrado (parseInt) -- documenta el comportamiento actual", () => {
    // parseInt("12abc") da 12, no NaN -- mismo comportamiento que ya tenía
    // el código original antes de extraer la función, no un cambio nuevo.
    expect(validarGafete("12abc", true)).toEqual({ valido: true, numero: 12 });
  });
});
