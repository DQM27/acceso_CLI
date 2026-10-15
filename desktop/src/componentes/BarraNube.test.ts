import { describe, expect, it } from "vitest";
import { descripcion } from "./BarraNube.logica";

describe("descripcion (BarraNube)", () => {
  it("sin red del SO, manda 'sin conexión' sin importar el estado del canal", () => {
    expect(descripcion("SUBSCRIBED", false).texto).toBe("Sin conexión — modo offline");
    expect(descripcion(null, false).texto).toBe("Sin conexión — modo offline");
  });

  it("sólo parpadea el punto cuando algo está caído", () => {
    expect(descripcion(null, false).parpadea).toBe(true);
    expect(descripcion("CLOSED", true).parpadea).toBe(true);
    expect(descripcion("SUBSCRIBED", true).parpadea).toBe(false);
    expect(descripcion(null, true).parpadea).toBe(false);
  });

  it("con red y canal suscrito, 'En línea'", () => {
    expect(descripcion("SUBSCRIBED", true)).toEqual({
      texto: "En línea",
      color: "var(--exito)",
      parpadea: false,
    });
  });

  it("con red y estado null (todavía sin intentar), 'Conectando…'", () => {
    expect(descripcion(null, true)).toEqual({
      texto: "Conectando…",
      color: "var(--muted)",
      parpadea: false,
    });
  });

  it("con red pero canal caído (error/timeout/cerrado), 'Sin conexión en vivo'", () => {
    for (const estado of ["CHANNEL_ERROR", "TIMED_OUT", "CLOSED"] as const) {
      expect(descripcion(estado, true)).toEqual({
        texto: "Sin conexión en vivo",
        color: "var(--error)",
        parpadea: true,
      });
    }
  });
});
