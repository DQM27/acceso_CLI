import { describe, expect, it } from "vitest";
import { requierePraind, textoTipoIngreso } from "./contratistas";

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

describe("textoTipoIngreso", () => {
  it("en mayusculas y con las palabras separadas", () => {
    expect(textoTipoIngreso("Praind")).toBe("PRAIND");
    expect(textoTipoIngreso("InHouse")).toBe("IN HOUSE");
    expect(textoTipoIngreso("PorCorreo")).toBe("POR CORREO");
    expect(textoTipoIngreso("Swat")).toBe("SWAT");
    expect(textoTipoIngreso(null)).toBe("\u2014");
  });
});
