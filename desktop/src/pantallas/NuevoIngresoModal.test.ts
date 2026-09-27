import { describe, expect, it } from "vitest";
import { avisosContratista, numeroDeGafete } from "./NuevoIngresoModal.logica";

describe("avisosContratista (chips del buscador)", () => {
  const hoy = "2026-09-23";
  type Datos = Parameters<typeof avisosContratista>[0];
  const base: Datos = { tiene_ingreso_activo: false, tiene_acceso: true, fecha_vencimiento_praind: "2027-01-01" };
  const textos = (c: Partial<Datos>) =>
    avisosContratista({ ...base, ...c }, hoy).map((aviso) => aviso.texto);

  it("sin nada que avisar, no hay chips", () => {
    expect(textos({})).toEqual([]);
  });

  it("avisa adentro, sin acceso y PRAIND vencido, en ese orden", () => {
    expect(
      textos({ tiene_ingreso_activo: true, tiene_acceso: false, fecha_vencimiento_praind: "2026-09-22" }),
    ).toEqual(["Adentro", "Sin acceso", "PRAIND vencido"]);
  });

  it("el PRAIND que vence hoy todavía no está vencido; sin fecha no avisa", () => {
    expect(textos({ fecha_vencimiento_praind: "2026-09-23" })).toEqual([]);
    expect(textos({ fecha_vencimiento_praind: null })).toEqual([]);
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
