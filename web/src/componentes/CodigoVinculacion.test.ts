import { describe, expect, it } from "vitest";
import { contenidoQr, tiempoRestante } from "./CodigoVinculacion.logica";

describe("código de vinculación en el panel", () => {
  it("el QR lleva sólo el código, sin separadores, con el esquema de la app", () => {
    expect(contenidoQr("K7QM-R4XT-2P")).toBe("brisas-acceso://vincular?codigo=K7QMR4XT2P");
  });

  it("muestra la cuenta regresiva en m:ss y en h:mm:ss si falta más de una hora", () => {
    const ahora = Date.parse("2026-09-29T12:00:00Z");
    expect(tiempoRestante("2026-09-29T12:14:59Z", ahora)).toBe("14:59");
    expect(tiempoRestante("2026-09-29T13:30:05Z", ahora)).toBe("1:30:05");
  });

  it("devuelve null cuando el código ya venció", () => {
    const ahora = Date.parse("2026-09-29T12:00:00Z");
    expect(tiempoRestante("2026-09-29T12:00:00Z", ahora)).toBeNull();
    expect(tiempoRestante("2026-09-29T11:59:00Z", ahora)).toBeNull();
  });
});
