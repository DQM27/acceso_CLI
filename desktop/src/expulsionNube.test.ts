import { describe, expect, it } from "vitest";
import { mensajeExpulsion, motivoSiEsParaEsteEquipo } from "./expulsionNube";

const equipo = { dispositivo_id: "d1", huella: "h1" };

describe("aviso dispositivo_expulsado", () => {
  it("ignora avisos de otro dispositivo del sitio", () => {
    expect(motivoSiEsParaEsteEquipo({ dispositivo_id: "d2", motivo: "revocado" }, equipo)).toBeNull();
  });

  it("aplica revocación y suspensión de este dispositivo", () => {
    expect(motivoSiEsParaEsteEquipo({ dispositivo_id: "d1", motivo: "revocado", huella: "h1" }, equipo)).toBe(
      "revocado",
    );
    expect(motivoSiEsParaEsteEquipo({ dispositivo_id: "d1", motivo: "suspendido", huella: null }, equipo)).toBe(
      "suspendido",
    );
  });

  it("al re-vincular, sólo expulsa al equipo cuya huella quedó fuera", () => {
    expect(motivoSiEsParaEsteEquipo({ dispositivo_id: "d1", motivo: "revinculado", huella: "h1" }, equipo)).toBe(
      "revinculado",
    );
    expect(
      motivoSiEsParaEsteEquipo({ dispositivo_id: "d1", motivo: "revinculado", huella: "h-vieja" }, equipo),
    ).toBeNull();
  });

  it("descarta avisos malformados o con motivos desconocidos", () => {
    expect(motivoSiEsParaEsteEquipo(null, equipo)).toBeNull();
    expect(motivoSiEsParaEsteEquipo({ dispositivo_id: "d1", motivo: "otro" }, equipo)).toBeNull();
  });

  it("tiene un mensaje para cada motivo", () => {
    for (const motivo of ["revocado", "suspendido", "revinculado"] as const) {
      expect(mensajeExpulsion(motivo)).not.toHaveLength(0);
    }
  });
});
