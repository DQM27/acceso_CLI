import { describe, expect, it } from "vitest";
import { mensajeConflictoGafete } from "./conflictoGafete";

describe("aviso de gafete ya asignado en otro dispositivo", () => {
  const base = { nombre: "Juan Pérez", gafete_numero: 12, fecha_hora: "2026-09-30T14:00:00Z" };

  it("nombra el tipo de movimiento que no quedó registrado", () => {
    expect(mensajeConflictoGafete({ ...base, tipo: "contratista" }, "08:00")).toBe(
      "El ingreso de Juan Pérez con gafete 12 (08:00) no quedó registrado en la nube — otro dispositivo de este sitio ya lo tiene asignado.",
    );
    expect(mensajeConflictoGafete({ ...base, tipo: "proveedor" }, "08:00")).toContain(
      "El ingreso del proveedor Juan Pérez con gafete 12 (08:00)",
    );
    expect(mensajeConflictoGafete({ ...base, tipo: "provisional_kof" }, "08:00")).toContain(
      "El préstamo del gafete provisional 12 a Juan Pérez (08:00)",
    );
    expect(mensajeConflictoGafete({ ...base, tipo: "por_correo" }, "08:00")).toContain(
      "El ingreso por correo de Juan Pérez con gafete de visita 12 (08:00)",
    );
  });
});
