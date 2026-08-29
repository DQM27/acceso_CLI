import { describe, expect, it } from "vitest";
import { mensajeResultado } from "./historial";
import type { MovimientoIngresoResumen } from "./historial";

function movimiento(
  resultado_acceso: MovimientoIngresoResumen["resultado_acceso"],
): MovimientoIngresoResumen {
  return {
    registro_id: 1,
    contratista_id: 1,
    cedula: "1-0847-0293",
    contratista_nombre: "Marlon Quesada",
    empresa_nombre: "Constructora del Valle",
    tipo_ingreso: "Praind",
    medio_ingreso: "Caminando",
    fecha_hora_ingreso: "2027-03-08T12:00:00Z",
    fecha_hora_salida: null,
    gafete_numero: null,
    usuario_ingreso_nombre: "root",
    usuario_salida_nombre: null,
    resultado_acceso,
    motivo_resultado: null,
    reglas_version: 1,
    empresa_activa_snapshot: true,
  };
}

describe("mensajeResultado", () => {
  it("Permitido", () => {
    expect(mensajeResultado(movimiento("Permitido"))).toBe("Permitido");
  });

  it("Migrado", () => {
    expect(mensajeResultado(movimiento("Migrado"))).toBe("Migrado");
  });

  it("PermitidoConAdvertencia distingue el motivo", () => {
    expect(
      mensajeResultado(
        movimiento({ PermitidoConAdvertencia: "PraindProximoVencer" }),
      ),
    ).toBe("PRAIND próximo a vencer");
    expect(
      mensajeResultado(movimiento({ PermitidoConAdvertencia: "DatosReconstruidos" })),
    ).toBe("Datos reconstruidos");
  });
});
