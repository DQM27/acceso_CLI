import { describe, expect, it } from "vitest";
import { colorEstado, textoEstado } from "./Activos";
import type { IngresoActivoResumen } from "../api";

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
