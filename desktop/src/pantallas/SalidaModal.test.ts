import { describe, expect, it } from "vitest";
import { coincideTexto } from "./SalidaModal";
import type { FilaActiva, FilaLocal, FilaRemota } from "../api";

function activo(overrides: Partial<FilaLocal> = {}): FilaActiva {
  return {
    origen: "local",
    registro_id: 1,
    contratista_id: 1,
    cedula: "1-0847-0293",
    contratista_nombre: "Marlon Quesada",
    empresa_nombre: "Constructora del Valle",
    tipo_ingreso: "Praind",
    medio_ingreso: "Caminando",
    fecha_hora_ingreso: "2027-03-08T12:00:00Z",
    gafete_numero: null,
    usuario_ingreso_nombre: "root",
    resultado_registrado: "Permitido",
    resultado_acceso: "Permitido",
    ...overrides,
  };
}

function remoto(overrides: Partial<FilaRemota> = {}): FilaActiva {
  return {
    origen: "remoto",
    uuid_remoto: "uuid-remoto",
    registro_id: null,
    contratista_id: null,
    cedula: "1-0847-0293",
    contratista_nombre: "Marlon Quesada",
    empresa_nombre: "Constructora del Valle",
    tipo_ingreso: "Praind",
    medio_ingreso: "Caminando",
    fecha_hora_ingreso: "2027-03-08T12:00:00Z",
    gafete_numero: null,
    usuario_ingreso_nombre: "Op PC",
    resultado_registrado: null,
    resultado_acceso: null,
    ...overrides,
  };
}

describe("coincideTexto", () => {
  it("busca por nombre, sin importar mayúsculas", () => {
    expect(coincideTexto(activo({ contratista_nombre: "Marlon Quesada" }), "marlon")).toBe(true);
    expect(coincideTexto(activo({ contratista_nombre: "Marlon Quesada" }), "MARLON")).toBe(true);
  });

  it("busca por cédula", () => {
    expect(coincideTexto(activo({ cedula: "1-0847-0293" }), "0847")).toBe(true);
  });

  it("coincidencia parcial en cualquier posición", () => {
    expect(coincideTexto(activo({ contratista_nombre: "Marlon Quesada" }), "esada")).toBe(true);
  });

  it("sin coincidencia en ninguno de los dos campos", () => {
    expect(coincideTexto(activo({ contratista_nombre: "Marlon Quesada", cedula: "1-0847-0293" }), "yuliana")).toBe(
      false,
    );
  });

  // Antes este modal sólo buscaba entre locales (`IngresoActivoResumen`) --
  // una fila abierta por otro dispositivo del mismo sitio no aparecía acá
  // aunque sí se podía cerrar desde la grilla de Activos. `coincideTexto`
  // debe funcionar igual sobre las dos formas.
  it("encuentra por nombre a una persona activa en otro dispositivo del mismo sitio", () => {
    expect(coincideTexto(remoto({ contratista_nombre: "Persona Remota" }), "remota")).toBe(true);
  });

  it("una fila remota sin cédula (nube no la mandó) no rompe la búsqueda", () => {
    expect(coincideTexto(remoto({ cedula: null }), "0847")).toBe(false);
    expect(coincideTexto(remoto({ cedula: null, contratista_nombre: "Persona Remota" }), "remota")).toBe(true);
  });
});
