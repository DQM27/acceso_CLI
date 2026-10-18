import { describe, expect, it } from "vitest";
import { activoParaGafete, agruparPorGafete, coincideTexto, textoGafete } from "./SalidaModal.logica";
import type { ActivoUnificado } from "../api/activosUnificados";

function activo(overrides: Partial<ActivoUnificado> = {}): ActivoUnificado {
  return {
    clase: "contratista",
    clave: "contratista-local-1",
    cedula: "1-0847-0293",
    nombre: "Marlon Quesada",
    detalle: "Constructora del Valle",
    tipo: "PRAIND",
    medio: "CAMINANDO",
    gafete_numero: null,
    fecha_hora_ingreso: "2027-03-08T12:00:00Z",
    usuario_ingreso_nombre: "root",
    nivel_permanencia: "Normal",
    fuente: undefined as never,
    ...overrides,
  };
}

describe("coincideTexto", () => {
  it("busca por nombre, sin importar mayúsculas", () => {
    expect(coincideTexto(activo({ nombre: "Marlon Quesada" }), "marlon")).toBe(true);
    expect(coincideTexto(activo({ nombre: "Marlon Quesada" }), "MARLON")).toBe(true);
  });

  it("busca por cédula", () => {
    expect(coincideTexto(activo({ cedula: "1-0847-0293" }), "0847")).toBe(true);
  });

  it("coincidencia parcial en cualquier posición", () => {
    expect(coincideTexto(activo({ nombre: "Marlon Quesada" }), "esada")).toBe(true);
  });

  it("busca por lo que acompaña al nombre: empresa, motivo o código de empleado", () => {
    expect(coincideTexto(activo({ detalle: "Constructora del Valle" }), "valle")).toBe(true);
    expect(coincideTexto(activo({ clase: "kof", cedula: null, detalle: "Cód. 5866173" }), "5866")).toBe(true);
  });

  it("sin coincidencia en ninguno de los campos", () => {
    expect(coincideTexto(activo({ nombre: "Marlon Quesada", cedula: "1-0847-0293" }), "yuliana")).toBe(false);
  });

  it("un KOF sin cédula ni detalle no rompe la búsqueda", () => {
    expect(coincideTexto(activo({ cedula: null, detalle: null, nombre: "Encargado" }), "0847")).toBe(false);
    expect(coincideTexto(activo({ cedula: null, detalle: null, nombre: "Encargado" }), "encarg")).toBe(true);
  });
});

describe("textoGafete", () => {
  it("muestra el numero en mayusculas o S/G sin gafete", () => {
    expect(textoGafete(2)).toBe("GAFETE 2");
    expect(textoGafete(null)).toBe("S/G");
  });
});

describe("agruparPorGafete", () => {
  it("guarda juntos a los que comparten número, de clases distintas", () => {
    const contratista = activo({ clave: "c-1", gafete_numero: 1 });
    const proveedor = activo({ clase: "proveedor", clave: "p-1", gafete_numero: 1 });
    const otro = activo({ clave: "c-2", gafete_numero: 2 });
    const mapa = agruparPorGafete([contratista, proveedor, otro]);
    expect(mapa.get(1)).toEqual([contratista, proveedor]);
    expect(mapa.get(2)).toEqual([otro]);
  });

  it("ignora a los que no tienen gafete", () => {
    expect(agruparPorGafete([activo({ gafete_numero: null })]).size).toBe(0);
  });
});

describe("activoParaGafete", () => {
  const contratista = activo({ clave: "c-1", gafete_numero: 1 });
  const proveedor = activo({ clase: "proveedor", clave: "p-1", gafete_numero: 1 });

  it("si nadie tiene ese número, no hay a quién dar la salida", () => {
    expect(activoParaGafete(undefined, undefined)).toBeUndefined();
    expect(activoParaGafete([], undefined)).toBeUndefined();
  });

  it("si lo tiene uno solo, es ése sin pedir elegir", () => {
    expect(activoParaGafete([contratista], undefined)).toBe(contratista);
  });

  it("si lo tienen varios y no se eligió, no adivina: ninguno", () => {
    expect(activoParaGafete([contratista, proveedor], undefined)).toBeUndefined();
  });

  it("si lo tienen varios, da la salida al elegido", () => {
    expect(activoParaGafete([contratista, proveedor], "p-1")).toBe(proveedor);
    expect(activoParaGafete([contratista, proveedor], "c-1")).toBe(contratista);
  });

  it("una elección que ya no corresponde a ninguno no cuenta", () => {
    expect(activoParaGafete([contratista, proveedor], "kof-9")).toBeUndefined();
  });
});
