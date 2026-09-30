import { describe, expect, it } from "vitest";
import { ESTADOS_ACCESO, nivelPraind, textoEstadoPraind } from "./Contratistas.logica";

describe("textoEstadoPraind", () => {
  it("dice cuánto falta o cuánto hace, en singular y plural", () => {
    expect(textoEstadoPraind({ estado_praind: "VENCIDA", dias_para_vencer: -1 })).toBe("Vencida hace 1 día");
    expect(textoEstadoPraind({ estado_praind: "VENCIDA", dias_para_vencer: -12 })).toBe("Vencida hace 12 días");
    expect(textoEstadoPraind({ estado_praind: "POR_VENCER", dias_para_vencer: 0 })).toBe("Vence hoy");
    expect(textoEstadoPraind({ estado_praind: "POR_VENCER", dias_para_vencer: 5 })).toBe("Vence en 5 días");
  });

  it("nombra los estados sin fecha", () => {
    expect(textoEstadoPraind({ estado_praind: "VIGENTE", dias_para_vencer: 200 })).toBe("Vigente");
    expect(textoEstadoPraind({ estado_praind: "NO_REQUIERE", dias_para_vencer: null })).toBe("No requiere");
    expect(textoEstadoPraind({ estado_praind: "SIN_REGISTRO", dias_para_vencer: null })).toBe("Sin registrar");
  });
});

describe("niveles", () => {
  it("vencida o sin registrar bloquea, por vencer avisa, el resto no se resalta", () => {
    expect(nivelPraind("VENCIDA")).toBe("bloqueo");
    expect(nivelPraind("SIN_REGISTRO")).toBe("bloqueo");
    expect(nivelPraind("POR_VENCER")).toBe("aviso");
    expect(nivelPraind("VIGENTE")).toBeNull();
    expect(nivelPraind("NO_REQUIERE")).toBeNull();
  });

  it("sólo PERMITIDO es ok y sólo la advertencia avisa", () => {
    expect(ESTADOS_ACCESO.PERMITIDO.nivel).toBe("ok");
    expect(ESTADOS_ACCESO.PERMITIDO_CON_ADVERTENCIA.nivel).toBe("aviso");
    for (const estado of ["PRAIND_VENCIDO", "PRAIND_NO_REGISTRADO", "SIN_ACCESO", "EMPRESA_INACTIVA"] as const) {
      expect(ESTADOS_ACCESO[estado].nivel).toBe("bloqueo");
    }
  });
});
