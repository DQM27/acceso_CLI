import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeTauri = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeTauri }));
const registrarLlamada = vi.fn();
let activa = false;
vi.mock("../telemetria", () => ({
  registrarLlamada,
  telemetriaActiva: () => activa,
}));

const { invoke } = await import("./invocar");

describe("invoke medido", () => {
  beforeEach(() => {
    invokeTauri.mockReset();
    registrarLlamada.mockReset();
  });

  it("sin telemetría reenvía exactamente los argumentos y no mide", async () => {
    activa = false;
    invokeTauri.mockResolvedValue(42);
    await expect(invoke("listar_empresas")).resolves.toBe(42);
    expect(invokeTauri).toHaveBeenCalledWith("listar_empresas");
    expect(invokeTauri.mock.calls[0]).toHaveLength(1);
    expect(registrarLlamada).not.toHaveBeenCalled();
  });

  it("con telemetría anota el nombre y si falló, y propaga el error", async () => {
    activa = true;
    invokeTauri.mockResolvedValueOnce("ok").mockRejectedValueOnce(new Error("sin red"));
    await expect(invoke("sincronizar_con_nube", { alcance: 1 })).resolves.toBe("ok");
    await expect(invoke("sincronizar_con_nube")).rejects.toThrow("sin red");
    expect(invokeTauri).toHaveBeenNthCalledWith(1, "sincronizar_con_nube", { alcance: 1 });
    expect(registrarLlamada.mock.calls.map(([nombre, , ok]) => [nombre, ok])).toEqual([
      ["sincronizar_con_nube", true],
      ["sincronizar_con_nube", false],
    ]);
  });
});
