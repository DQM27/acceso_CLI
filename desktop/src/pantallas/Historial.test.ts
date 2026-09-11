import { describe, expect, it } from "vitest";
import { filaDesdeRemoto, textoDispositivo } from "./Historial";
import type { MovimientoHistorialRemoto } from "../api";

function remoto(overrides: Partial<MovimientoHistorialRemoto> = {}): MovimientoHistorialRemoto {
  return {
    uuid: "uuid-1",
    cedula: "1-0847-0293",
    contratista_nombre: "Marlon Quesada",
    empresa_nombre: "Constructora del Valle",
    tipo_ingreso: "PRAIND",
    medio_ingreso: "CAMINANDO",
    fecha_hora_ingreso: "2027-03-08T12:00:00Z",
    fecha_hora_salida: null,
    gafete_numero: null,
    usuario_ingreso_nombre: "root",
    usuario_salida_nombre: null,
    dispositivo_entrada_tipo: "pc",
    ...overrides,
  };
}

describe("textoDispositivo", () => {
  it("pc y mobile dan su ícono", () => {
    expect(textoDispositivo("pc")).toBe("💻");
    expect(textoDispositivo("mobile")).toBe("📱");
  });

  it("null muestra un guión, sin inventar nada", () => {
    expect(textoDispositivo(null)).toBe("—");
  });

  it("cualquier otro valor se muestra tal cual", () => {
    expect(textoDispositivo("tablet")).toBe("tablet");
  });
});

describe("filaDesdeRemoto", () => {
  it("marca origen 'remoto' y registro_id/contratista_id en null", () => {
    const fila = filaDesdeRemoto(remoto());
    expect(fila.origen).toBe("remoto");
    expect(fila.registro_id).toBeNull();
    expect(fila.contratista_id).toBeNull();
  });

  it("usuario_ingreso_nombre null se reemplaza por guión, a diferencia de los demás campos", () => {
    const fila = filaDesdeRemoto(remoto({ usuario_ingreso_nombre: null }));
    expect(fila.usuario_ingreso_nombre).toBe("—");
  });

  it("copia cédula/empresa/gafete tal cual, sin transformar", () => {
    const fila = filaDesdeRemoto(remoto({ cedula: "2-1111-2222", gafete_numero: 7 }));
    expect(fila.cedula).toBe("2-1111-2222");
    expect(fila.gafete_numero).toBe(7);
  });
});
