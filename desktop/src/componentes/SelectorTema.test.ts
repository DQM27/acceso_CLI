import { afterEach, describe, expect, it, vi } from "vitest";
import { leerTema, siguienteTema, temaDelSistema } from "./SelectorTema";

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
    expect(temaDelSistema()).toBe("dark");
  });

  it("refleja prefers-color-scheme: light", () => {
    mockearSistema(false);
    expect(temaDelSistema()).toBe("light");
  });

  it("si matchMedia falla, cae a light en vez de romper", () => {
    romperMatchMedia();
    expect(temaDelSistema()).toBe("light");
  });
});

describe("leerTema", () => {
  it("usa lo guardado en localStorage si es válido", () => {
    localStorage.setItem(CLAVE_TEMA, "dark");
    mockearSistema(false);
    expect(leerTema()).toBe("dark");
  });

  it("sin nada guardado, cae al tema del sistema", () => {
    mockearSistema(true);
    expect(leerTema()).toBe("dark");
  });

  it("un valor guardado inválido se ignora, cae al tema del sistema", () => {
    localStorage.setItem(CLAVE_TEMA, "basura");
    mockearSistema(false);
    expect(leerTema()).toBe("light");
  });

  it("reconoce Tokyo Night guardado", () => {
    localStorage.setItem(CLAVE_TEMA, "tokyo-night");
    mockearSistema(false);
    expect(leerTema()).toBe("tokyo-night");
  });
});

describe("siguienteTema", () => {
  it("recorre claro → oscuro → Tokyo Night → claro", () => {
    expect(siguienteTema("light")).toBe("dark");
    expect(siguienteTema("dark")).toBe("tokyo-night");
    expect(siguienteTema("tokyo-night")).toBe("light");
  });
});
