import { describe, expect, it } from "vitest";
import type { AgendaVisitaResumen, MovimientoVisitaActivoResumen } from "../api";
import type { FilaCorreoActiva, HistorialIngresoCorreoRemoto } from "../api/correo";
import { unirAdentro, unirHistorial, visitasEsperadasHoy } from "./Visitas.logica";

function cita(cambios: Partial<AgendaVisitaResumen> = {}): AgendaVisitaResumen {
  return {
    cita_id: 1,
    cedula: "1-0847-0293",
    nombre: "ANA MORA",
    empresa: null,
    placa_vehiculo: null,
    motivo: "Reunión",
    anfitrion_nombre: "Daniel",
    fecha_desde: "2026-10-05",
    fecha_hasta: "2026-10-05",
    hora_estimada: null,
    estado: "Vigente",
    ...cambios,
  };
}

describe("visitasEsperadasHoy", () => {
  const hoy = "2026-10-05";

  it("sólo vigentes de hoy, sin las de otro día ni las canceladas", () => {
    const filas = visitasEsperadasHoy(
      [
        cita({ cita_id: 1, cedula: "100" }),
        cita({ cita_id: 2, cedula: "200", fecha_desde: "2026-10-06", fecha_hasta: "2026-10-06" }),
        cita({ cita_id: 3, cedula: "300", estado: "Cancelada" }),
        cita({ cita_id: 4, cedula: "400", fecha_desde: "2026-10-03", fecha_hasta: "2026-10-07" }),
      ],
      [],
      [],
      hoy,
    );
    expect(filas.map((fila) => fila.cita_id)).toEqual([1, 4]);
  });

  it("quien ya está adentro (por cualquier vía) no aparece, aunque la cédula venga escrita distinto", () => {
    const filas = visitasEsperadasHoy([cita({ cedula: "1-0847-0293" })], [{ cedula: "108470293" }], [], hoy);
    expect(filas).toEqual([]);
  });

  it("por hora estimada, sin hora al final, y quien ya salió hoy queda último y marcado", () => {
    const filas = visitasEsperadasHoy(
      [
        cita({ cita_id: 1, cedula: "100", nombre: "SIN HORA" }),
        cita({ cita_id: 2, cedula: "200", hora_estimada: "14:00" }),
        cita({ cita_id: 3, cedula: "300", hora_estimada: "08:30" }),
        cita({ cita_id: 4, cedula: "400", hora_estimada: "07:00" }),
      ],
      [],
      [{ cedula: "400", fecha_hora_salida: "2026-10-05T15:00:00Z" }],
      hoy,
    );
    expect(filas.map((fila) => fila.cita_id)).toEqual([3, 2, 1, 4]);
    expect(filas[3]?.llegada).toEqual({ tipo: "salio", hora: "2026-10-05T15:00:00Z" });
    expect(filas[0]?.llegada).toEqual({ tipo: "sin_llegar" });
  });

  it("una persona con dos citas hoy sale una sola vez", () => {
    const filas = visitasEsperadasHoy([cita({ cita_id: 1 }), cita({ cita_id: 2 })], [], [], hoy);
    expect(filas).toHaveLength(1);
  });
});

describe("unirAdentro y unirHistorial", () => {
  const visita: MovimientoVisitaActivoResumen = {
    id: 7,
    cedula: "100",
    nombre: "ANA",
    empresa: "ACME",
    gafete_numero: 3,
    fecha_hora_entrada: "2026-10-05T14:00:00Z",
    anfitrion_nombre: "Daniel",
    motivo: "Reunión",
  };
  const correo: FilaCorreoActiva = {
    origen: "remoto",
    uuid_remoto: "u-1",
    id: null,
    cedula: "200",
    nombre: "LUIS",
    motivo: "Entrevista RH",
    placa: "BCD123",
    gafete_numero: 5,
    fecha_hora_ingreso: "2026-10-05T15:00:00Z",
    usuario_ingreso_nombre: "Guarda",
  };

  it("adentro junta las dos, la más reciente primero, y sabe por dónde darles la salida", () => {
    const filas = unirAdentro([visita], [correo]);
    expect(filas.map((fila) => [fila.origen, fila.cedula])).toEqual([
      ["POR_CORREO", "200"],
      ["AGENDADA", "100"],
    ]);
    expect(filas[0]?.fuente).toEqual({ tipo: "correo", fila: correo });
    expect(filas[1]?.fuente).toEqual({ tipo: "visita", id: 7 });
    expect(new Set(filas.map((fila) => fila.clave)).size).toBe(2);
  });

  it("el historial junta las dos con su origen", () => {
    const historialCorreo: HistorialIngresoCorreoRemoto = {
      uuid: "c-1",
      cedula: "200",
      nombre: "LUIS",
      motivo: "Entrevista RH",
      placa: null,
      gafete_numero: 5,
      fecha_hora_ingreso: "2026-10-04T15:00:00Z",
      fecha_hora_salida: "2026-10-04T16:00:00Z",
      usuario_ingreso_nombre: "Guarda",
      usuario_salida_nombre: "Guarda",
    };
    const filas = unirHistorial(
      [
        {
          uuid: "v-1",
          cedula: "100",
          nombre: "ANA",
          empresa: null,
          anfitrion_nombre: "Daniel",
          motivo: null,
          gafete_numero: null,
          fecha_hora_entrada: "2026-10-05T14:00:00Z",
          fecha_hora_salida: null,
          usuario_entrada_nombre: "Guarda",
          usuario_salida_nombre: null,
        },
      ],
      [historialCorreo],
    );
    expect(filas.map((fila) => fila.origen)).toEqual(["AGENDADA", "POR_CORREO"]);
    expect(filas[1]?.fecha_hora_salida).toBe("2026-10-04T16:00:00Z");
  });
});
