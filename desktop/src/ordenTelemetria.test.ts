import { describe, expect, it } from "vitest";
import { esOrdenDeEsteEquipo } from "./ordenTelemetria";

describe("esOrdenDeEsteEquipo", () => {
  it("acepta la orden de este equipo", () => {
    expect(esOrdenDeEsteEquipo({ dispositivo_id: "d1", version: 3, grupos: [] }, "d1")).toBe(true);
  });

  it("descarta la orden de otro equipo del sitio", () => {
    expect(esOrdenDeEsteEquipo({ dispositivo_id: "d2", version: 3, grupos: [] }, "d1")).toBe(false);
  });

  it("descarta lo que no es un aviso con equipo", () => {
    expect(esOrdenDeEsteEquipo(null, "d1")).toBe(false);
    expect(esOrdenDeEsteEquipo(undefined, "d1")).toBe(false);
    expect(esOrdenDeEsteEquipo("d1", "d1")).toBe(false);
    expect(esOrdenDeEsteEquipo({}, "d1")).toBe(false);
    expect(esOrdenDeEsteEquipo({ dispositivo_id: 1 }, "1")).toBe(false);
  });
});
