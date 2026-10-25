import { describe, expect, it } from "vitest";
import { contenidoQr, detalleEvento, tiempoRestante } from "./CodigoVinculacion.logica";

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

describe("detalle de eventos de seguridad", () => {
  it("la sesión movida dice quién era y de qué unidad se le cerró", () => {
    const detalle = { cedula: "900000301", nombre: "OPERADOR UNO", sitio_anterior: "Brisas" };
    expect(detalleEvento("sesion_en_otra_unidad", detalle)).toBe("OPERADOR UNO (900000301): se cerró su sesión en Brisas");
  });

  it("la sesión en duda dice quién era y en qué otra unidad sigue", () => {
    const detalle = { cedula: "900000301", nombre: "OPERADOR UNO", sitio_otro: "Brisas" };
    expect(detalleEvento("sesion_en_duda", detalle)).toBe(
      "OPERADOR UNO (900000301): sigue también con sesión en Brisas",
    );
  });

  it("los demás eventos o un detalle malformado no agregan texto", () => {
    expect(detalleEvento("codigo_usado", { cedula: "1" })).toBeNull();
    expect(detalleEvento("sesion_en_otra_unidad", null)).toBeNull();
    expect(detalleEvento("sesion_en_otra_unidad", {})).toBeNull();
  });

  it("una reconexión dice quién la autorizó", () => {
    expect(detalleEvento("reconexion_autorizada", { autorizado_por: "admin@example.com" })).toBe(
      "por admin@example.com",
    );
    expect(detalleEvento("reconexion_autorizada", {})).toBeNull();
  });

  it("el canje dice si cambió de unidad y qué registros se descartaron", () => {
    expect(
      detalleEvento("reconexion_canjeada", { sitio_anterior: "s1", sitio_nuevo: "s1", pendientes_descartados: 0 }),
    ).toBe("en la misma unidad");
    expect(
      detalleEvento("reconexion_canjeada", {
        sitio_anterior: "s1",
        sitio_nuevo: "s2",
        pendientes_descartados: 3,
        confirmado_por: "Ana Mora",
      }),
    ).toBe("en otra unidad; se descartaron 3 registros sin subir (confirmó Ana Mora)");
    expect(
      detalleEvento("reconexion_canjeada", { sitio_anterior: "s1", sitio_nuevo: "s2", pendientes_descartados: 1 }),
    ).toBe("en otra unidad; se descartaron 1 registro sin subir");
  });
});
