import { describe, expect, it } from "vitest";
import {
  AgregadorBloqueos,
  AgregadorLlamadas,
  AgregadorRealtime,
  latenciaDesde,
  percentil,
  tipoDeError,
} from "./telemetriaCalculos";

describe("percentil", () => {
  it("rango más cercano, igual que en el teléfono", () => {
    expect(percentil([], 0.5)).toBeNull();
    expect(percentil([10, 20, 30, 40], 0.5)).toBe(20);
    expect(percentil([10, 20, 30, 40], 0.9)).toBe(40);
  });
});

describe("AgregadorLlamadas", () => {
  it("resume por comando y se vacía", () => {
    const llamadas = new AgregadorLlamadas();
    llamadas.registrar("sincronizar_con_nube", 500, true);
    llamadas.registrar("sincronizar_con_nube", 900, false);
    llamadas.registrar("listar_ingresos_activos", 3.26, true);

    const filas = llamadas.vaciar();
    expect(filas).toContainEqual({
      nombre: "sincronizar_con_nube",
      llamadas: 2,
      errores: 1,
      p50_ms: 500,
      p90_ms: 900,
      max_ms: 900,
      total_ms: 1400,
    });
    expect(filas.find((f) => f.nombre === "listar_ingresos_activos")?.p50_ms).toBe(3.3);
    expect(llamadas.vaciar()).toEqual([]);
  });
});

describe("AgregadorRealtime", () => {
  it("sin actividad no informa nada", () => {
    expect(new AgregadorRealtime().vaciar()).toBeNull();
  });

  it("cuenta conexión, avisos, ecos, aplicados y latencias con los campos del teléfono", () => {
    const realtime = new AgregadorRealtime();
    realtime.desfaseReloj(-120);
    realtime.conectando();
    realtime.suscrito(730);
    realtime.aviso("ingresos", 2048, false, 180);
    realtime.aviso("ingresos", 1024, true, 220);
    realtime.aplicado(true, 16.4);
    realtime.terminado("renovacion", 30 * 60_000);

    const resumen = realtime.vaciar();
    expect(resumen).toMatchObject({
      intentos_conexion: 1,
      ms_hasta_suscribir_p50: 730,
      fin_de_conexion: { renovacion: 1 },
      minutos_conectado_max: 30,
      avisos_por_tabla: { ingresos: 1 },
      avisos: 1,
      ecos_propios: 1,
      kb_recibidos: 3,
      aplicados: 1,
      no_aplicados: 0,
      latencia_ms_min: 180,
      latencia_ms_max: 220,
      latencia_corregida: true,
      desfase_reloj_ms: -120,
    });
    expect(realtime.vaciar()).toBeNull();
  });
});

describe("AgregadorBloqueos", () => {
  it("resume las tareas largas", () => {
    const bloqueos = new AgregadorBloqueos();
    expect(bloqueos.vaciar()).toBeNull();
    bloqueos.registrar(60);
    bloqueos.registrar(250);
    expect(bloqueos.vaciar()).toEqual({
      tareas_largas: 2,
      ms_bloqueado: 310,
      ms_p50: 60,
      ms_max: 250,
      mayores_a_200ms: 1,
    });
  });
});

describe("latenciaDesde", () => {
  it("corrige con el desfase de esta PC contra el servidor", () => {
    const servidor = "2026-09-29T03:35:00.000Z";
    const ahora = Date.parse(servidor) + 30_400;
    // La PC va 30 s adelantada: la latencia real son 400 ms.
    expect(latenciaDesde(servidor, ahora, 30_000)).toBe(400);
    expect(latenciaDesde(servidor, ahora, null)).toBe(30_400);
    expect(latenciaDesde("no es fecha", ahora, 0)).toBeNull();
  });
});

describe("tipoDeError", () => {
  it("sólo el tipo, nunca el mensaje", () => {
    expect(tipoDeError(new TypeError("cédula 1-2345-6789"))).toBe("TypeError");
    expect(tipoDeError("texto con datos")).toBe("string");
    expect(tipoDeError(undefined)).toBe("undefined");
  });
});
