import { describe, expect, it } from "vitest";
import {
  colorEstado,
  fechaLocalYMD,
  textoEstado,
  textoFechaDDMMYYYY,
  textoHora,
  textoMedio,
} from "./Activos";
import type { IngresoActivoResumen } from "../api";

describe("textoMedio", () => {
  it("Vehiculo -> Vehículo, cualquier otro -> Caminando", () => {
    expect(textoMedio("Vehiculo")).toBe("Vehículo");
    expect(textoMedio("Caminando")).toBe("Caminando");
  });
});

// Se arma la fecha con getters LOCALES directos (no parseando un string
// UTC fijo), y se compara contra lo que el propio runner calcularía para
// esa misma fecha local — así el test no depende de en qué huso horario
// corre CI, pero sigue agarrando un bug real de padding/orden de campos.
describe("fechaLocalYMD / textoHora — agnósticas al huso horario del runner", () => {
  const fecha = new Date(2027, 2, 8, 14, 5); // 8 de marzo de 2027, 14:05 local
  const iso = fecha.toISOString();

  it("fechaLocalYMD arma año-mes-día con padding, en hora local", () => {
    const esperado = `${fecha.getFullYear()}-${String(fecha.getMonth() + 1).padStart(2, "0")}-${String(
      fecha.getDate(),
    ).padStart(2, "0")}`;
    expect(fechaLocalYMD(iso)).toBe(esperado);
  });

  it("textoHora es de 24 horas, con padding", () => {
    const esperado = fecha.toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      hour12: false,
    });
    expect(textoHora(iso)).toBe(esperado);
  });
});

describe("textoFechaDDMMYYYY", () => {
  it("reordena YYYY-MM-DD a DD/MM/YYYY", () => {
    expect(textoFechaDDMMYYYY("2027-03-08")).toBe("08/03/2027");
  });
});

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
