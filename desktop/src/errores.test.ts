import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

import { describirError, fijarPantallaErrores, reportarErrorJs } from "./errores";

describe("describirError", () => {
  it("un Error da su nombre, su mensaje y su traza", () => {
    const error = new TypeError("x is undefined");
    expect(describirError(error, "error")).toEqual({ titulo: "TypeError: x is undefined", traza: error.stack });
  });

  it("marca el origen de las promesas y del render", () => {
    expect(describirError(new Error("boom"), "promesa").titulo).toBe("Promesa rechazada: Error: boom");
    expect(describirError(new Error("boom"), "render").titulo).toBe("Render: Error: boom");
  });

  it("lo que no es un Error se describe como texto, sin traza", () => {
    expect(describirError("se cayó", "error")).toEqual({ titulo: "se cayó", traza: "" });
    expect(describirError({ codigo: 7 }, "promesa")).toEqual({ titulo: 'Promesa rechazada: {"codigo":7}', traza: "" });
    expect(describirError(undefined, "error").titulo).toBe("undefined");
    expect(describirError("", "error").titulo).toBe("(sin mensaje)");
  });

  it("recorta los títulos gigantes", () => {
    expect(describirError("x".repeat(1000), "error").titulo).toHaveLength(300);
  });
});

describe("envío al núcleo", () => {
  beforeEach(() => {
    mocks.invoke.mockReset();
    mocks.invoke.mockResolvedValue(undefined);
  });

  it("reporta con título y traza", () => {
    reportarErrorJs(new RangeError("fuera"), "error");
    expect(mocks.invoke).toHaveBeenCalledWith("reportar_error_js", {
      titulo: "RangeError: fuera",
      traza: expect.stringContaining("RangeError"),
    });
  });

  it("si el backend falla, reportar no lanza", async () => {
    mocks.invoke.mockRejectedValue(new Error("sin backend"));
    expect(() => reportarErrorJs("x", "error")).not.toThrow();
    await Promise.resolve();
  });

  it("avisa la pantalla sólo cuando cambia", () => {
    fijarPantallaErrores("Ingresos");
    fijarPantallaErrores("Ingresos");
    fijarPantallaErrores("Historial");
    expect(mocks.invoke.mock.calls).toEqual([
      ["errores_pantalla", { nombre: "Ingresos" }],
      ["errores_pantalla", { nombre: "Historial" }],
    ]);
  });
});
