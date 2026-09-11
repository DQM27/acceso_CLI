import { describe, expect, it } from "vitest";
import { columnasPara } from "./Contratistas";

function campos(actorRol: Parameters<typeof columnasPara>[0]): (string | undefined)[] {
  return columnasPara(actorRol).map((c) => c.field);
}

describe("columnasPara (Contratistas)", () => {
  it("Operador no ve la columna 'Acceso' -- se delega al panel web", () => {
    expect(campos("Operador")).not.toContain("tiene_acceso");
  });

  it("Administrador y Root sí ven 'Acceso'", () => {
    expect(campos("Administrador")).toContain("tiene_acceso");
    expect(campos("Root")).toContain("tiene_acceso");
  });

  it("las columnas base siempre están, sin importar el rol", () => {
    for (const rol of ["Operador", "Administrador", "Root"] as const) {
      expect(campos(rol)).toEqual(
        expect.arrayContaining([
          "cedula",
          "nombre",
          "empresa_nombre",
          "tipo_ingreso",
          "fecha_vencimiento_praind",
          "es_personal_ruta",
        ]),
      );
    }
  });
});
