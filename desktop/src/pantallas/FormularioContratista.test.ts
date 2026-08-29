import { describe, expect, it } from "vitest";
import { esquema } from "./FormularioContratista";

function valores(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    cedula: "108470293",
    nombre: "Marlon Quesada",
    empresa_id: "5",
    tipo_ingreso: "PorCorreo",
    fecha_vencimiento_praind: "",
    es_personal_ruta: false,
    tiene_acceso: true,
    ...overrides,
  };
}

describe("esquema de FormularioContratista", () => {
  it("acepta valores válidos", () => {
    expect(esquema.safeParse(valores()).success).toBe(true);
  });

  it("cédula vacía o con letras no pasa", () => {
    expect(esquema.safeParse(valores({ cedula: "" })).success).toBe(false);
    expect(esquema.safeParse(valores({ cedula: "108-470293" })).success).toBe(false);
  });

  it("nombre vacío o con números no pasa", () => {
    expect(esquema.safeParse(valores({ nombre: "" })).success).toBe(false);
    expect(esquema.safeParse(valores({ nombre: "Marlon2" })).success).toBe(false);
  });

  it("nombre con acentos, apóstrofe o guión sí pasa", () => {
    expect(esquema.safeParse(valores({ nombre: "José O'Neill Pérez-Ruiz" })).success).toBe(true);
  });

  it("empresa_id vacío no pasa", () => {
    expect(esquema.safeParse(valores({ empresa_id: "" })).success).toBe(false);
  });

  it("tipo_ingreso fuera del enum no pasa", () => {
    expect(esquema.safeParse(valores({ tipo_ingreso: "Otro" })).success).toBe(false);
  });

  it("PRAIND requerido: Praind sin fecha no pasa, con fecha sí", () => {
    const sinFecha = esquema.safeParse(
      valores({ tipo_ingreso: "Praind", fecha_vencimiento_praind: "" }),
    );
    expect(sinFecha.success).toBe(false);

    const conFecha = esquema.safeParse(
      valores({ tipo_ingreso: "Praind", fecha_vencimiento_praind: "2027-03-08" }),
    );
    expect(conFecha.success).toBe(true);
  });

  it("PorCorreo sin ser de ruta no exige fecha PRAIND", () => {
    expect(
      esquema.safeParse(
        valores({ tipo_ingreso: "PorCorreo", es_personal_ruta: false, fecha_vencimiento_praind: "" }),
      ).success,
    ).toBe(true);
  });

  it("personal de ruta exige fecha PRAIND aunque el tipo sea PorCorreo", () => {
    expect(
      esquema.safeParse(
        valores({ tipo_ingreso: "PorCorreo", es_personal_ruta: true, fecha_vencimiento_praind: "" }),
      ).success,
    ).toBe(false);
  });
});
