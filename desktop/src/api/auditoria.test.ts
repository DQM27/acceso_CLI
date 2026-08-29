import { describe, expect, it } from "vitest";
import { etiquetaCampo, etiquetaEntidad, valorPresentable } from "./auditoria";

describe("etiquetaEntidad", () => {
  it.each([
    ["Contratista", "Contratista"],
    ["Empresa", "Empresa"],
    ["Usuario", "Usuario"],
  ] as const)("%s -> %s", (entidad, esperado) => {
    expect(etiquetaEntidad(entidad)).toBe(esperado);
  });
});

describe("etiquetaCampo", () => {
  it("traduce las claves crudas conocidas", () => {
    expect(etiquetaCampo("cedula")).toBe("Cédula");
    expect(etiquetaCampo("fecha_vencimiento_praind")).toBe("Vencimiento PRAIND");
    expect(etiquetaCampo("password")).toBe("Contraseña");
  });

  it("una clave desconocida se muestra tal cual — nunca en blanco", () => {
    expect(etiquetaCampo("campo_nuevo_del_nucleo")).toBe("campo_nuevo_del_nucleo");
  });
});

describe("valorPresentable", () => {
  it("password es un marcador sin valores, siempre un guión", () => {
    expect(valorPresentable("password", null)).toBe("—");
    expect(valorPresentable("password", "cualquier-hash")).toBe("—");
  });

  it("null en fecha_vencimiento_praind se lee como 'Sin fecha', no como guión", () => {
    expect(valorPresentable("fecha_vencimiento_praind", null)).toBe("Sin fecha");
  });

  it("null en cualquier otro campo es un guión", () => {
    expect(valorPresentable("nombre", null)).toBe("—");
  });

  it("reformatea la fecha de ISO a DD/MM/YYYY sin pasar por Date", () => {
    expect(valorPresentable("fecha_vencimiento_praind", "2027-03-08")).toBe("08/03/2027");
  });

  it("traduce los valores crudos de tipo_ingreso", () => {
    expect(valorPresentable("tipo_ingreso", "IN_HOUSE")).toBe("IN HOUSE");
    expect(valorPresentable("tipo_ingreso", "POR_CORREO")).toBe("POR CORREO");
  });

  it("traduce los valores crudos de tiene_acceso", () => {
    expect(valorPresentable("tiene_acceso", "HABILITADO")).toBe("Habilitado");
    expect(valorPresentable("tiene_acceso", "DESHABILITADO")).toBe("Deshabilitado");
  });

  it("traduce SI/NO para es_personal_ruta y activo", () => {
    expect(valorPresentable("es_personal_ruta", "SI")).toBe("Sí");
    expect(valorPresentable("activo", "NO")).toBe("No");
  });

  it("un valor sin regla especial se muestra tal cual", () => {
    expect(valorPresentable("nombre", "Marlon Quesada")).toBe("Marlon Quesada");
  });
});
