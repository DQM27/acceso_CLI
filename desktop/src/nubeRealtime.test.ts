import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { iniciarRealtimeNube } from "./nubeRealtime";
import { solicitarSincronizacionNube } from "./eventosNube";

// El canal privado real (reconexión, JWT, Presence) vive del lado Rust
// desde 2026-09-26 (`desktop/src-tauri/src/realtime_nube.rs`, ver
// `benchmarks/realtime-rust/HANDOFF.md`) -- estos tests sólo verifican el
// puente hacia React: que arranca/para el comando correcto y que traduce
// los dos eventos Tauri a los mismos callbacks que ya usaban
// `App.tsx`/`BarraNube.tsx`.
const mocks = vi.hoisted(() => ({
  iniciarComando: vi.fn(),
  detenerComando: vi.fn(),
  sincronizar: vi.fn(),
  listen: vi.fn(),
}));
vi.mock("./api/nube", () => ({
  iniciarRealtimeNubeComando: mocks.iniciarComando,
  detenerRealtimeNubeComando: mocks.detenerComando,
  sincronizarConNube: mocks.sincronizar,
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));

type Escucha<T> = (evento: { payload: T }) => void;
const escuchas = new Map<string, Escucha<unknown>>();
let detener: (() => void) | undefined;
const resumen = { enviados: 0 };

beforeEach(() => {
  vi.useFakeTimers();
  vi.clearAllMocks();
  escuchas.clear();
  mocks.iniciarComando.mockResolvedValue(undefined);
  mocks.detenerComando.mockResolvedValue(undefined);
  mocks.sincronizar.mockResolvedValue(resumen);
  mocks.listen.mockImplementation((evento: string, callback: Escucha<unknown>) => {
    escuchas.set(evento, callback);
    return Promise.resolve(vi.fn());
  });
});
afterEach(() => {
  detener?.();
  detener = undefined;
  vi.useRealTimers();
});

function emitirEstado(estado: string) {
  escuchas.get("nube://estado_realtime")?.({ payload: estado });
}
function emitirSincronizado(payload: unknown) {
  escuchas.get("nube://sincronizado_realtime")?.({ payload });
}

describe("puente de Realtime con el backend Rust", () => {
  it("arranca el canal real con la cédula/nombre del usuario", () => {
    detener = iniciarRealtimeNube({ usuario: { cedula: "123", nombre: "Ana" } });
    expect(mocks.iniciarComando).toHaveBeenCalledWith("123", "Ana");
  });

  it("traduce nube://estado_realtime al callback onEstado", async () => {
    const onEstado = vi.fn();
    detener = iniciarRealtimeNube({ onEstado });
    await vi.advanceTimersByTimeAsync(0);
    emitirEstado("SUBSCRIBED");
    expect(onEstado).toHaveBeenCalledWith("SUBSCRIBED");
  });

  it("traduce nube://sincronizado_realtime al callback onSincronizado", async () => {
    const onSincronizado = vi.fn();
    detener = iniciarRealtimeNube({ onSincronizado });
    await vi.advanceTimersByTimeAsync(0);
    emitirSincronizado(resumen);
    expect(onSincronizado).toHaveBeenCalledWith(resumen);
  });

  it("debounce de un cambio local dispara sincronizar_con_nube una sola vez", async () => {
    detener = iniciarRealtimeNube();
    await vi.advanceTimersByTimeAsync(0);
    for (let i = 0; i < 5; i++) solicitarSincronizacionNube();
    await vi.advanceTimersByTimeAsync(600);
    expect(mocks.sincronizar).toHaveBeenCalledTimes(1);
  });

  it("detener() para el canal real y deja de reaccionar a eventos", async () => {
    const onEstado = vi.fn();
    const onSincronizado = vi.fn();
    detener = iniciarRealtimeNube({ onEstado, onSincronizado });
    await vi.advanceTimersByTimeAsync(0);
    detener();
    expect(mocks.detenerComando).toHaveBeenCalledTimes(1);

    emitirEstado("SUBSCRIBED");
    emitirSincronizado(resumen);
    solicitarSincronizacionNube();
    await vi.advanceTimersByTimeAsync(600);
    expect(onEstado).not.toHaveBeenCalled();
    expect(onSincronizado).not.toHaveBeenCalled();
    expect(mocks.sincronizar).not.toHaveBeenCalled();
  });
});
