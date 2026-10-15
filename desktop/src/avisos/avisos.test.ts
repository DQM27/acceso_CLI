import { beforeEach, describe, expect, it, vi } from "vitest";

const sonner = vi.hoisted(() => ({
  success: vi.fn(),
  info: vi.fn(),
  warning: vi.fn(),
  error: vi.fn(),
  promise: vi.fn(),
  dismiss: vi.fn(),
}));

vi.mock("sonner", () => ({ toast: sonner }));

import { avisos } from "./avisos";
import { duracionDe, textoDeError, tituloPorDefecto } from "./avisos.logica";

describe("avisos.logica", () => {
  it("cada tipo tiene un título por defecto", () => {
    expect(tituloPorDefecto("exito")).toBe("Listo");
    expect(tituloPorDefecto("info")).toBe("Aviso");
    expect(tituloPorDefecto("advertencia")).toBe("Atención");
    expect(tituloPorDefecto("error")).toBe("No se pudo");
  });

  it("errores y advertencias duran más que una confirmación", () => {
    expect(duracionDe("error", false)).toBeGreaterThan(duracionDe("exito", false));
    expect(duracionDe("advertencia", false)).toBeGreaterThan(duracionDe("info", false));
  });

  it("un aviso persistente no se va solo, sea cual sea su tipo", () => {
    for (const tipo of ["exito", "info", "advertencia", "error"] as const) {
      expect(duracionDe(tipo, true)).toBe(Infinity);
    }
  });

  it("el texto de un error string se deja tal cual", () => {
    expect(textoDeError("sin permiso")).toBe("sin permiso");
  });

  it("el texto de un Error o de cualquier otra cosa sale de String()", () => {
    expect(textoDeError(new Error("sin conexión"))).toBe("Error: sin conexión");
    expect(textoDeError(42)).toBe("42");
  });
});

describe("avisos", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("exito usa el título 'Listo' y pone el mensaje como descripción", () => {
    avisos.exito("Ingreso registrado.");
    expect(sonner.success).toHaveBeenCalledWith(
      "Listo",
      expect.objectContaining({ description: "Ingreso registrado.", duration: 4500 }),
    );
  });

  it("cada tipo llama a su propio método del motor", () => {
    avisos.info("a");
    avisos.advertencia("b");
    avisos.error("c");
    expect(sonner.info).toHaveBeenCalledTimes(1);
    expect(sonner.warning).toHaveBeenCalledTimes(1);
    expect(sonner.error).toHaveBeenCalledTimes(1);
  });

  it("error acepta un Error tal como llega de catch", () => {
    avisos.error(new Error("falló"));
    expect(sonner.error).toHaveBeenCalledWith(
      "No se pudo",
      expect.objectContaining({ description: "Error: falló" }),
    );
  });

  it("un título propio reemplaza al del tipo", () => {
    avisos.info("Se descarga sola.", { titulo: "Versión 1.9.0 disponible" });
    expect(sonner.info).toHaveBeenCalledWith("Versión 1.9.0 disponible", expect.anything());
  });

  it("persistente: no se va solo y trae cruz para cerrarlo", () => {
    avisos.error("Te expulsaron.", { persistente: true });
    expect(sonner.error).toHaveBeenCalledWith(
      "No se pudo",
      expect.objectContaining({ duration: Infinity, closeButton: true }),
    );
  });

  it("la acción se traduce a la forma del motor", () => {
    const alAccionar = vi.fn();
    avisos.info("x", { accion: { etiqueta: "Actualizar", alAccionar } });
    const [, configuracion] = sonner.info.mock.calls[0] as [string, { action: { label: string } }];
    expect(configuracion.action.label).toBe("Actualizar");
  });

  it("progreso muestra el error tal cual si no se da un texto propio", () => {
    avisos.progreso(Promise.resolve(1), { cargando: "Exportando…", exito: "Listo." });
    const [, textos] = sonner.promise.mock.calls[0] as [
      Promise<number>,
      { loading: string; error: (causa: unknown) => string },
    ];
    expect(textos.loading).toBe("Exportando…");
    expect(textos.error("disco lleno")).toBe("disco lleno");
  });

  it("progreso usa el texto de error propio cuando se da", () => {
    avisos.progreso(Promise.resolve(1), {
      cargando: "…",
      exito: "ok",
      error: (causa) => `No se pudo actualizar: ${String(causa)}`,
    });
    const [, textos] = sonner.promise.mock.calls[0] as [
      Promise<number>,
      { error: (causa: unknown) => string },
    ];
    expect(textos.error("red")).toBe("No se pudo actualizar: red");
  });
});
