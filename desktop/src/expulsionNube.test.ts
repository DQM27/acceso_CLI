import { describe, expect, it } from "vitest";
import { esExpulsionDeEsteEquipo } from "./expulsionNube";

describe("aviso dispositivo_expulsado", () => {
  it("sólo es para el dispositivo que nombra", () => {
    expect(esExpulsionDeEsteEquipo({ dispositivo_id: "d1", motivo: "revocado" }, "d1")).toBe(true);
    expect(esExpulsionDeEsteEquipo({ dispositivo_id: "d2", motivo: "revocado" }, "d1")).toBe(false);
  });

  it("descarta avisos malformados", () => {
    expect(esExpulsionDeEsteEquipo(null, "d1")).toBe(false);
    expect(esExpulsionDeEsteEquipo("d1", "d1")).toBe(false);
    expect(esExpulsionDeEsteEquipo({}, "d1")).toBe(false);
  });
});
