import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { iniciarRealtimeNube } from "./nubeRealtime";
import { solicitarSincronizacionNube } from "./eventosNube";

const mocks = vi.hoisted(() => ({
  sesion: vi.fn(),
  sincronizar: vi.fn(),
  sincronizarCambios: vi.fn(),
  aplicarCambio: vi.fn(),
  crear: vi.fn(),
}));
vi.mock("./api/nube", () => ({
  aplicarCambioNube: mocks.aplicarCambio,
  sesionRealtimeNube: mocks.sesion,
  sincronizarConNube: mocks.sincronizar,
  sincronizarCambiosNube: mocks.sincronizarCambios,
}));
vi.mock("@supabase/supabase-js", () => ({ createClient: mocks.crear }));

interface CanalPrueba {
  estado: (estado: string, error?: Error) => void;
  aviso: (mensaje: {
    payload: { dispositivo_id: string; table?: string; operation?: string; registro?: unknown };
  }) => void;
  accessToken: () => Promise<string>;
}
const canales: CanalPrueba[] = [];
let detener: (() => void) | undefined;
const sesion = {
  base_url: "https://ejemplo.supabase.co", apikey: "publicable-prueba",
  access_token: "jwt-dispositivo", expires_in: 3600,
  sitio_id: "sitio-a", dispositivo_id: "equipo-a", topic: "sitio:sitio-a",
};
const resumen = { enviados: 0 };

beforeEach(() => {
  vi.useFakeTimers();
  vi.clearAllMocks();
  canales.length = 0;
  mocks.sesion.mockResolvedValue(sesion);
  mocks.sincronizar.mockResolvedValue(resumen);
  mocks.sincronizarCambios.mockResolvedValue(resumen);
  mocks.aplicarCambio.mockResolvedValue(true);
  mocks.crear.mockImplementation((_url, _key, opciones) => {
    const control: CanalPrueba = { estado: () => {}, aviso: () => {}, accessToken: opciones.accessToken };
    const canal = {
      on: vi.fn((_tipo, _filtro, callback) => { control.aviso = callback; return canal; }),
      subscribe: vi.fn((callback) => { control.estado = callback; return canal; }),
      track: vi.fn().mockResolvedValue(undefined),
    };
    canales.push(control);
    return {
      channel: vi.fn((_topic, config) => { expect(config.config.private).toBe(true); return canal; }),
      realtime: { setAuth: vi.fn().mockResolvedValue(undefined), disconnect: vi.fn() },
      removeChannel: vi.fn(() => { control.estado("CLOSED"); return Promise.resolve("ok"); }),
    };
  });
});
afterEach(() => { detener?.(); detener = undefined; vi.useRealTimers(); });

async function iniciar() {
  detener = iniciarRealtimeNube();
  await vi.advanceTimersByTimeAsync(0);
  return canales[0];
}

describe("sincronización por Realtime", () => {
  it("un aviso con la fila se aplica al instante y la sincronización por tabla sigue detrás", async () => {
    const recargas = vi.fn();
    window.addEventListener("nube:cambio-en-vivo", recargas);
    const canal = await iniciar();
    const payload = { dispositivo_id: "equipo-b", table: "ingresos", operation: "INSERT", registro: { id: "u1" } };

    canal.aviso({ payload });
    await vi.advanceTimersByTimeAsync(0);

    expect(mocks.aplicarCambio).toHaveBeenCalledWith(payload);
    expect(recargas).toHaveBeenCalledTimes(1);
    await vi.advanceTimersByTimeAsync(600);
    expect(mocks.sincronizarCambios).toHaveBeenCalledWith(["ingresos"]);
    window.removeEventListener("nube:cambio-en-vivo", recargas);
  });

  it("un aviso sin datos no intenta aplicar nada", async () => {
    const canal = await iniciar();
    canal.aviso({ payload: { dispositivo_id: "equipo-b", table: "empresas", operation: "UPDATE" } });
    await vi.advanceTimersByTimeAsync(600);
    expect(mocks.aplicarCambio).not.toHaveBeenCalled();
    expect(mocks.sincronizarCambios).toHaveBeenCalledWith(["empresas"]);
  });

  it("conserva el JWT del dispositivo cuando el SDK vuelve a pedirlo", async () => {
    const canal = await iniciar();
    expect(await canal.accessToken()).toBe("jwt-dispositivo");
    canal.estado("SUBSCRIBED");
    await vi.advanceTimersByTimeAsync(600);
    expect(mocks.sincronizar).toHaveBeenCalledTimes(1);
  });

  it("agrupa avisos remotos y evita sincronizar por el eco del propio dispositivo", async () => {
    const canal = await iniciar();
    canal.aviso({ payload: { dispositivo_id: "equipo-a" } });
    await vi.advanceTimersByTimeAsync(600);
    expect(mocks.sincronizar).not.toHaveBeenCalled();
    for (let i = 0; i < 5; i++) canal.aviso({ payload: { dispositivo_id: "equipo-b" } });
    await vi.advanceTimersByTimeAsync(600);
    expect(mocks.sincronizar).toHaveBeenCalledTimes(1);
  });

  it("conserva cambios que llegan mientras la sincronización sigue en curso", async () => {
    let resolver: (valor: unknown) => void = () => {};
    mocks.sincronizar.mockImplementationOnce(() => new Promise((resolve) => { resolver = resolve; }));
    const canal = await iniciar();
    solicitarSincronizacionNube();
    await vi.advanceTimersByTimeAsync(600);
    canal.aviso({ payload: { dispositivo_id: "equipo-b" } });
    await vi.advanceTimersByTimeAsync(600);
    expect(mocks.sincronizar).toHaveBeenCalledTimes(1);
    resolver(resumen);
    await vi.advanceTimersByTimeAsync(600);
    expect(mocks.sincronizar).toHaveBeenCalledTimes(2);
  });

  it("renueva el token sin reconectar otra vez por el cierre del canal anterior", async () => {
    await iniciar();
    mocks.sesion.mockResolvedValue({ ...sesion, access_token: "jwt-renovado" });
    await vi.advanceTimersByTimeAsync(3_542_000);
    expect(canales).toHaveLength(2);
    expect(await canales[1].accessToken()).toBe("jwt-renovado");
    await vi.advanceTimersByTimeAsync(10_000);
    expect(canales).toHaveLength(2);
  });

  it("no abre un cliente si se cierra sesión mientras espera autenticación", async () => {
    let resolver: (valor: unknown) => void = () => {};
    mocks.sesion.mockImplementationOnce(() => new Promise((resolve) => { resolver = resolve; }));
    detener = iniciarRealtimeNube();
    detener();
    resolver(sesion);
    await vi.advanceTimersByTimeAsync(0);
    expect(mocks.crear).not.toHaveBeenCalled();
  });

  it("ignora avisos y cambios locales después de detenerse", async () => {
    const canal = await iniciar();
    detener?.();
    solicitarSincronizacionNube();
    canal.aviso({ payload: { dispositivo_id: "equipo-b" } });
    canal.estado("CLOSED");
    await vi.advanceTimersByTimeAsync(60_000);
    expect(mocks.sincronizar).not.toHaveBeenCalled();
    expect(canales).toHaveLength(1);
  });

  it("un aviso con tabla sincroniza sólo esa parte y junta las tablas de la ráfaga", async () => {
    const canal = await iniciar();
    canal.aviso({ payload: { dispositivo_id: "equipo-b", table: "ingresos" } });
    canal.aviso({ payload: { dispositivo_id: "equipo-b", table: "gafetes" } });
    canal.aviso({ payload: { dispositivo_id: "equipo-b", table: "ingresos" } });
    await vi.advanceTimersByTimeAsync(600);
    expect(mocks.sincronizar).not.toHaveBeenCalled();
    expect(mocks.sincronizarCambios).toHaveBeenCalledTimes(1);
    expect(mocks.sincronizarCambios).toHaveBeenCalledWith(["ingresos", "gafetes"]);
  });

  it("un cambio local en la misma ráfaga fuerza la sincronización completa", async () => {
    const canal = await iniciar();
    canal.aviso({ payload: { dispositivo_id: "equipo-b", table: "empresas" } });
    solicitarSincronizacionNube();
    await vi.advanceTimersByTimeAsync(600);
    expect(mocks.sincronizar).toHaveBeenCalledTimes(1);
    expect(mocks.sincronizarCambios).not.toHaveBeenCalled();
  });

  it("la reconexión del canal sincroniza todo para recuperar lo perdido", async () => {
    const canal = await iniciar();
    canal.aviso({ payload: { dispositivo_id: "equipo-b", table: "empresas" } });
    canal.estado("SUBSCRIBED");
    await vi.advanceTimersByTimeAsync(600);
    expect(mocks.sincronizar).toHaveBeenCalledTimes(1);
    expect(mocks.sincronizarCambios).not.toHaveBeenCalled();
  });

  it("las tablas que llegan durante una sincronización en curso van en la siguiente", async () => {
    let resolver: (valor: unknown) => void = () => {};
    mocks.sincronizarCambios.mockImplementationOnce(() => new Promise((resolve) => { resolver = resolve; }));
    const canal = await iniciar();
    canal.aviso({ payload: { dispositivo_id: "equipo-b", table: "ingresos" } });
    await vi.advanceTimersByTimeAsync(600);
    canal.aviso({ payload: { dispositivo_id: "equipo-b", table: "citas" } });
    await vi.advanceTimersByTimeAsync(600);
    expect(mocks.sincronizarCambios).toHaveBeenCalledTimes(1);
    resolver(resumen);
    await vi.advanceTimersByTimeAsync(600);
    expect(mocks.sincronizarCambios).toHaveBeenCalledTimes(2);
    expect(mocks.sincronizarCambios).toHaveBeenLastCalledWith(["citas"]);
  });
});
