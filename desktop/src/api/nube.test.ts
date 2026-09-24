import { afterEach, describe, expect, it, vi } from "vitest";
import { EVENTO_CAMBIO_LOCAL_NUBE } from "../eventosNube";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn(() => Promise.resolve()) }));

import { cerrarIngresoProveedorRemoto, cerrarIngresoRemoto } from "./nube";

// Bug real 2026-09-23: cerrar desde la PC un ingreso abierto en el celular
// no refrescaba el historial -- el aviso Realtime de ese cambio trae el id
// de la propia PC y se descarta, así que el cierre tiene que pedir la
// sincronización por su cuenta.
describe("cierres remotos piden sincronizar", () => {
  const escuchar = vi.fn();

  afterEach(() => {
    window.removeEventListener(EVENTO_CAMBIO_LOCAL_NUBE, escuchar);
    escuchar.mockClear();
  });

  it("cerrarIngresoRemoto", async () => {
    window.addEventListener(EVENTO_CAMBIO_LOCAL_NUBE, escuchar);
    await cerrarIngresoRemoto("uuid-1");
    expect(escuchar).toHaveBeenCalledTimes(1);
  });

  it("cerrarIngresoProveedorRemoto", async () => {
    window.addEventListener(EVENTO_CAMBIO_LOCAL_NUBE, escuchar);
    await cerrarIngresoProveedorRemoto("uuid-2");
    expect(escuchar).toHaveBeenCalledTimes(1);
  });
});
