import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { iniciarRealtimeNube } from "./nubeRealtime";
import { solicitarSincronizacionNube } from "./eventosNube";

const mocks = vi.hoisted(() => ({ sesion: vi.fn(), sincronizar: vi.fn(), crear: vi.fn() }));
vi.mock("./api/nube", () => ({ sesionRealtimeNube: mocks.sesion, sincronizarConNube: mocks.sincronizar }));
vi.mock("@supabase/supabase-js", () => ({ createClient: mocks.crear }));

interface CanalPrueba {
  estado: (estado: string, error?: Error) => void;
  aviso: (mensaje: { payload: { dispositivo_id: string } }) => void;
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
  mocks.crear.mockImplementation((_url, _key, opciones) => {
    const control: CanalPrueba = { estado: () => {}, aviso: () => {}, accessToken: opciones.accessToken };
    const canal = {
      on: vi.fn((_tipo, _filtro, callback) => { control.aviso = callback; return canal; }),
      subscribe: vi.fn((callback) => { control.estado = callback; return canal; }),
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
});
