import { describe, expect, it } from "vitest";
import { requierePraind } from "./contratistas";

describe("requierePraind", () => {
  it("personal de ruta siempre lo requiere, sin importar el tipo", () => {
    expect(requierePraind({ es_personal_ruta: true, tipo_ingreso: "PorCorreo" })).toBe(true);
    expect(requierePraind({ es_personal_ruta: true, tipo_ingreso: "Swat" })).toBe(true);
  });

  it.each(["Praind", "InHouse"] as const)("tipo %s lo requiere aunque no sea de ruta", (tipo) => {
    expect(requierePraind({ es_personal_ruta: false, tipo_ingreso: tipo })).toBe(true);
  });

  it.each(["PorCorreo", "Swat"] as const)("tipo %s no lo requiere si no es de ruta", (tipo) => {
    expect(requierePraind({ es_personal_ruta: false, tipo_ingreso: tipo })).toBe(false);
  });
});
