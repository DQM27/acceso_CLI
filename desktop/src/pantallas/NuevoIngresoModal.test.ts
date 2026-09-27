import { describe, expect, it } from "vitest";
import { avisosContratista, numeroDeGafete } from "./NuevoIngresoModal.logica";

describe("avisosContratista (chips del buscador)", () => {
  type Datos = Parameters<typeof avisosContratista>[0];
  const base: Datos = { tiene_ingreso_activo: false, aviso_acceso: null };
  const textos = (c: Partial<Datos>) => avisosContratista({ ...base, ...c }).map((aviso) => aviso.texto);

  it("sin nada que avisar, no hay chips", () => {
    expect(textos({})).toEqual([]);
  });

  it("muestra adentro y el aviso del núcleo tal cual, en ese orden", () => {
    expect(textos({ tiene_ingreso_activo: true, aviso_acceso: "ACCESO DENEGADO" })).toEqual([
      "Adentro",
      "ACCESO DENEGADO",
    ]);
  });
});

describe("numeroDeGafete", () => {
  it("sin gafete requerido o vacío es null (el núcleo decide si falta)", () => {
    expect(numeroDeGafete("basura", false)).toBeNull();
    expect(numeroDeGafete("   ", true)).toBeNull();
  });

  it("texto que no es número es undefined", () => {
    expect(numeroDeGafete("abc", true)).toBeUndefined();
  });

  it("número con espacios se convierte", () => {
    expect(numeroDeGafete("  7  ", true)).toBe(7);
  });
});
