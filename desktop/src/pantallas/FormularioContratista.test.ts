import { describe, expect, it } from "vitest";
import { esquema } from "./FormularioContratista.logica";

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

// Las reglas (cédula, nombre, PRAIND, personal de ruta) las prueba el
// núcleo (`tests/contratista_service.rs`); acá sólo queda lo del formulario.
describe("esquema de FormularioContratista", () => {
  it("acepta valores completos", () => {
    expect(esquema.safeParse(valores()).success).toBe(true);
  });

  it("sin empresa no pasa", () => {
    expect(esquema.safeParse(valores({ empresa_id: "" })).success).toBe(false);
  });

  it("tipo_ingreso fuera del enum no pasa", () => {
    expect(esquema.safeParse(valores({ tipo_ingreso: "Otro" })).success).toBe(
      false,
    );
  });

  it("no replica reglas del núcleo: cédula con guiones pasa al núcleo", () => {
    expect(esquema.safeParse(valores({ cedula: "108-470293" })).success).toBe(
      true,
    );
  });
});
