import { afterEach, describe, expect, it, vi } from "vitest";
import { CLAVE_TEMA_ULTIMO, leerTema, siguienteTema, temaDelSistema, temaInicial } from "./SelectorTema.logica";

const CLAVE_TEMA = "escritorio:tema";

// jsdom no implementa matchMedia -- se asigna directo (no `vi.spyOn`, que
// necesita una función existente para espiar) y se restaura a mano.
function mockearSistema(prefiereDark: boolean) {
  window.matchMedia = vi.fn().mockReturnValue({ matches: prefiereDark } as MediaQueryList);
}

function romperMatchMedia() {
  window.matchMedia = vi.fn(() => {
    throw new Error("no soportado en este entorno");
  });
}

afterEach(() => {
  localStorage.clear();
  vi.restoreAllMocks();
  // @ts-expect-error -- jsdom no lo define de entrada, se limpia el mock.
  delete window.matchMedia;
});

describe("temaDelSistema", () => {
  it("refleja prefers-color-scheme: dark", () => {
    mockearSistema(true);
    expect(temaDelSistema()).toBe("tokyo-night");
  });

  it("refleja prefers-color-scheme: light", () => {
    mockearSistema(false);
    expect(temaDelSistema()).toBe("tokyo-day");
  });

  it("si matchMedia falla, cae a Tokyo Day en vez de romper", () => {
    romperMatchMedia();
    expect(temaDelSistema()).toBe("tokyo-day");
  });
});

describe("leerTema", () => {
  it("usa lo guardado en localStorage si es válido", () => {
    localStorage.setItem(CLAVE_TEMA, "tokyo-night");
    mockearSistema(false);
    expect(leerTema()).toBe("tokyo-night");
  });

  it("sin nada guardado, cae al tema del sistema", () => {
    mockearSistema(true);
    expect(leerTema()).toBe("tokyo-night");
  });

  it("un valor guardado inválido se ignora, cae al tema del sistema", () => {
    localStorage.setItem(CLAVE_TEMA, "basura");
    mockearSistema(false);
    expect(leerTema()).toBe("tokyo-day");
  });

  it("lee el tema del usuario indicado", () => {
    localStorage.setItem(`u5:${CLAVE_TEMA}`, "tokyo-night");
    localStorage.setItem(`u6:${CLAVE_TEMA}`, "tokyo-day");
    mockearSistema(true);
    expect(leerTema(5)).toBe("tokyo-night");
    expect(leerTema(6)).toBe("tokyo-day");
  });

  it("las elecciones de antes (claro y oscuro) pasan a su Tokyo equivalente", () => {
    mockearSistema(true);
    localStorage.setItem(CLAVE_TEMA, "light");
    expect(leerTema()).toBe("tokyo-day");
    localStorage.setItem(CLAVE_TEMA, "dark");
    mockearSistema(false);
    expect(leerTema()).toBe("tokyo-night");
  });
});

describe("siguienteTema", () => {
  it("alterna Tokyo Day ↔ Tokyo Night", () => {
    expect(siguienteTema("tokyo-day")).toBe("tokyo-night");
    expect(siguienteTema("tokyo-night")).toBe("tokyo-day");
  });
});

describe("temaInicial (antes de iniciar sesión)", () => {
  it("usa el último tema elegido en esta PC", () => {
    mockearSistema(false);
    localStorage.setItem(CLAVE_TEMA_ULTIMO, "tokyo-night");
    expect(temaInicial()).toBe("tokyo-night");
  });

  it("sin nada guardado (o con algo inválido) toma el del sistema", () => {
    mockearSistema(true);
    expect(temaInicial()).toBe("tokyo-night");
    localStorage.setItem(CLAVE_TEMA_ULTIMO, "brisas");
    expect(temaInicial()).toBe("tokyo-night");
  });

  it("no lee el tema por usuario: es el de la PC, no el de nadie en particular", () => {
    mockearSistema(false);
    localStorage.setItem("u7:escritorio:tema", "tokyo-night");
    expect(temaInicial()).toBe("tokyo-day");
  });
});
