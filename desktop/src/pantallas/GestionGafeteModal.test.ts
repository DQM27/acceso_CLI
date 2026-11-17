import { describe, expect, it } from "vitest";
import { detalleLimpio, textoEstado, textoTipoIncidente } from "./GestionGafeteModal.logica";

describe("textoEstado", () => {
  it("traduce cada estado a su texto visible", () => {
    expect(textoEstado("Disponible")).toBe("Disponible");
    expect(textoEstado("Perdido")).toBe("Perdido");
  });
});

describe("detalleLimpio", () => {
  it("quita espacios y deja null si no queda nada", () => {
    expect(detalleLimpio("  recibí sin el 07 ")).toBe("recibí sin el 07");
    expect(detalleLimpio("   ")).toBeNull();
    expect(detalleLimpio("")).toBeNull();
  });
});

describe("textoTipoIncidente", () => {
  it("nombra cada reporte, incluidos los cierres viejos", () => {
    expect(textoTipoIncidente({ tipo: "Perdido", motivo_resolucion: null })).toBe("Reportado perdido");
    expect(textoTipoIncidente({ tipo: "Pagado", motivo_resolucion: null })).toBe("Pago registrado");
    expect(textoTipoIncidente({ tipo: "Recuperado", motivo_resolucion: null })).toBe("Recuperado");
    expect(textoTipoIncidente({ tipo: "Resuelto", motivo_resolucion: "Pagado" })).toBe(
      "Resuelto · pagado",
    );
    expect(textoTipoIncidente({ tipo: "Resuelto", motivo_resolucion: "Aparecido" })).toBe(
      "Resuelto · apareció",
    );
  });
});
