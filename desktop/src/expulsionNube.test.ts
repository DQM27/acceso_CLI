import { describe, expect, it } from "vitest";
import { esCierreDeEstaSesion, esExpulsionDeEsteEquipo } from "./expulsionNube";

describe("aviso sesion_cerrada", () => {
  it("sólo es para el usuario con sesión en este equipo", () => {
    expect(esCierreDeEstaSesion({ cedula: "900000301" }, "900000301")).toBe(true);
    expect(esCierreDeEstaSesion({ cedula: "900000302" }, "900000301")).toBe(false);
  });

  it("sin sesión o con avisos malformados no hace nada", () => {
    expect(esCierreDeEstaSesion({ cedula: "900000301" }, undefined)).toBe(false);
    expect(esCierreDeEstaSesion(null, "900000301")).toBe(false);
    expect(esCierreDeEstaSesion("900000301", "900000301")).toBe(false);
  });
});

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
