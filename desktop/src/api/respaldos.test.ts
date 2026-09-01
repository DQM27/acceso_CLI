import { describe, expect, it } from "vitest";
import { esValido, etiquetaTipoRespaldo, textoValidacion } from "./respaldos";

describe("etiquetaTipoRespaldo", () => {
  it.each([
    ["Manual", "Manual"],
    ["Automatico", "Automático"],
    ["PreMigracion", "Pre-migración"],
    ["PreRestauracion", "Pre-restauración"],
    ["PorFlag", "Por flag (CLI)"],
  ] as const)("%s -> %s", (tipo, esperado) => {
    expect(etiquetaTipoRespaldo(tipo)).toBe(esperado);
  });
});

describe("esValido", () => {
  it("Valido es válido", () => {
    expect(esValido({ Valido: { version_esquema: 15 } })).toBe(true);
  });

  it("Invalido y EsquemaIncompatible no son válidos", () => {
    expect(esValido({ Invalido: "boom" })).toBe(false);
    expect(esValido({ EsquemaIncompatible: { version_encontrada: 99 } })).toBe(false);
  });
});

describe("textoValidacion", () => {
  it("Valido muestra la versión de esquema", () => {
    expect(textoValidacion({ Valido: { version_esquema: 15 } })).toBe("Válido (esquema v15)");
  });

  it("Invalido incluye el detalle del núcleo", () => {
    expect(textoValidacion({ Invalido: "hay referencias de clave foránea inválidas" })).toBe(
      "No pasó la verificación: hay referencias de clave foránea inválidas",
    );
  });

  it("EsquemaIncompatible muestra la versión encontrada", () => {
    expect(textoValidacion({ EsquemaIncompatible: { version_encontrada: 99 } })).toBe(
      "Esquema futuro no reconocido (v99)",
    );
  });
});
