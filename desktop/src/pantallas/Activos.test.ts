import { describe, expect, it } from "vitest";
import { colorEstado, filaDesdeLocal, filaDesdeRemoto, textoEstado } from "./Activos";
import type { IngresoActivoResumen, IngresoRemoto } from "../api";

// fechaLocalYMD/textoHora/textoFechaDDMMYYYY se probaron en
// src/tiempo.test.ts, y textoMedio en api/ingresos.test.ts — Activos las
// importa de ahí, no las define más.

describe("textoEstado / colorEstado", () => {
  function fila(
    resultado_acceso: IngresoActivoResumen["resultado_acceso"],
  ): IngresoActivoResumen {
    return {
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
      resultado_acceso,
    };
  }

  it("Permitido: Al día, verde", () => {
    expect(textoEstado(fila("Permitido"))).toBe("Al día");
    expect(colorEstado(fila("Permitido"))).toBe("var(--exito)");
  });

  it("PermitidoConAdvertencia: PRAIND próximo a vencer, ámbar", () => {
    expect(textoEstado(fila("PermitidoConAdvertencia"))).toBe("PRAIND próximo a vencer");
    expect(colorEstado(fila("PermitidoConAdvertencia"))).toBe("var(--advertencia)");
  });

  it("Denegado: usa el mensaje del motivo, rojo", () => {
    expect(textoEstado(fila({ Denegado: "PraindVencido" }))).toBe(
      "Acceso denegado · PRAIND vencido",
    );
    expect(colorEstado(fila({ Denegado: "PraindVencido" }))).toBe("var(--error)");
  });
});

// Fusión con lo abierto por el otro dispositivo del mismo sitio
// (`docs/plan-persistencia-nube.md`) -- ninguna de las dos filas se
// confunde con la otra a la hora de decidir cómo cerrarla.
describe("filaDesdeLocal / filaDesdeRemoto", () => {
  it("una fila local conserva sus datos y queda marcada origen: local", () => {
    const item: IngresoActivoResumen = {
      registro_id: 7,
      contratista_id: 3,
      cedula: "1-0847-0293",
      contratista_nombre: "Marlon Quesada",
      empresa_nombre: "Constructora del Valle",
      tipo_ingreso: "Praind",
      medio_ingreso: "Caminando",
      fecha_hora_ingreso: "2027-03-08T12:00:00Z",
      gafete_numero: 10,
      usuario_ingreso_nombre: "root",
      resultado_registrado: "Permitido",
      resultado_acceso: "Permitido",
    };

    const resultado = filaDesdeLocal(item);

    expect(resultado).toMatchObject({
      origen: "local",
      registro_id: 7,
      contratista_nombre: "Marlon Quesada",
      estado_texto: "Al día",
    });
  });

  it("una fila remota no trae id local ni datos que la nube no tiene", () => {
    const remoto: IngresoRemoto = {
      uuid: "uuid-remoto",
      contratista_nombre: "Persona Remota",
      hora_entrada: "2027-03-08T08:00:00Z",
      usuario_entrada_nombre: "Op PC",
    };

    const resultado = filaDesdeRemoto(remoto);

    expect(resultado).toMatchObject({
      origen: "remoto",
      uuid_remoto: "uuid-remoto",
      registro_id: null,
      contratista_nombre: "Persona Remota",
      usuario_ingreso_nombre: "Op PC",
      estado_texto: "Otro dispositivo",
    });
  });

  it("una fila remota sin usuario de entrada muestra un guión, no vacío", () => {
    const remoto: IngresoRemoto = {
      uuid: "uuid-remoto",
      contratista_nombre: "Persona Remota",
      hora_entrada: "2027-03-08T08:00:00Z",
      usuario_entrada_nombre: null,
    };

    expect(filaDesdeRemoto(remoto).usuario_ingreso_nombre).toBe("—");
  });
});
