import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  listarMovimientosPagina,
  listarMovimientosParaExportar,
  listarUnidadesOperativas,
  palabrasDeBusqueda,
} from "./historial";

function mockConsulta(resultado: { data: unknown; error: unknown; count: number | null }) {
  const encadenable: Record<string, unknown> = {
    select: vi.fn(() => encadenable),
    order: vi.fn(() => encadenable),
    range: vi.fn(() => encadenable),
    gte: vi.fn(() => encadenable),
    lt: vi.fn(() => encadenable),
    in: vi.fn(() => encadenable),
    like: vi.fn(() => encadenable),
    or: vi.fn(() => encadenable),
    returns: vi.fn(() => encadenable),
    then: (resolver: (valor: typeof resultado) => void) => resolver(resultado),
  };
  return encadenable;
}

function filaVista(sobrescribir: Record<string, unknown> = {}) {
  return {
    id: "1",
    sitio_id: "s1",
    sitio_nombre: "Brisas",
    contratista_cedula: "001",
    contratista_nombre: "Alguien",
    empresa_nombre: "Brisas",
    tipo_ingreso: "PRAIND",
    medio_ingreso: "CAMINANDO",
    gafete_numero: 1,
    hora_entrada: "2026-08-20T14:30:00Z",
    hora_salida: null,
    usuario_entrada_nombre: "Quintana",
    usuario_salida_nombre: null,
    dispositivo_entrada_tipo: "pc",
    tipo_texto: "PRAIND",
    medio_texto: "CAMINANDO",
    ...sobrescribir,
  };
}

const mocks = vi.hoisted(() => ({ from: vi.fn() }));
vi.mock("../lib/supabase", () => ({ supabase: { from: mocks.from } }));

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(() => {
  vi.restoreAllMocks();
});

describe("palabrasDeBusqueda", () => {
  it("separa por espacios, pliega tildes, ignora vacíos y escapa \\, % y _ para like", () => {
    expect(palabrasDeBusqueda("  Pérez   100%_x  ")).toEqual(["perez", "100\\%\\_x"]);
    expect(palabrasDeBusqueda("   ")).toEqual([]);
  });
});

describe("listarMovimientosPagina", () => {
  it("lee la vista panel_movimientos y devuelve la página con el total de todas las páginas", async () => {
    const encadenable = mockConsulta({ data: [filaVista()], error: null, count: 4321 });
    mocks.from.mockReturnValue(encadenable);

    const resultado = await listarMovimientosPagina({ pagina: 0, tamano: 100 });

    expect(mocks.from).toHaveBeenCalledWith("panel_movimientos");
    expect(resultado.total).toBe(4321);
    expect(resultado.filas).toHaveLength(1);
    expect(resultado.filas[0].sitio_nombre).toBe("Brisas");
    expect(resultado.filas[0].medio_texto).toBe("CAMINANDO");
  });

  it("pide sólo el tramo de la página (base 0) y ordena por hora_entrada descendente con desempate por id", async () => {
    const encadenable = mockConsulta({ data: [], error: null, count: 0 });
    mocks.from.mockReturnValue(encadenable);

    await listarMovimientosPagina({ pagina: 2, tamano: 50 });

    expect(encadenable.range).toHaveBeenCalledWith(100, 149);
    expect(encadenable.order).toHaveBeenNthCalledWith(1, "hora_entrada", {
      ascending: false,
      nullsFirst: false,
    });
    expect(encadenable.order).toHaveBeenNthCalledWith(2, "id");
  });

  it("respeta el orden pedido", async () => {
    const encadenable = mockConsulta({ data: [], error: null, count: 0 });
    mocks.from.mockReturnValue(encadenable);

    await listarMovimientosPagina({
      pagina: 0,
      tamano: 50,
      orden: { campo: "empresa_nombre", descendente: false },
    });

    expect(encadenable.order).toHaveBeenNthCalledWith(1, "empresa_nombre", {
      ascending: true,
      nullsFirst: false,
    });
  });

  it("aplica fechas (Costa Rica), unidades y una condición like por cada palabra de la búsqueda", async () => {
    const encadenable = mockConsulta({ data: [], error: null, count: 0 });
    mocks.from.mockReturnValue(encadenable);

    await listarMovimientosPagina({
      desde: "2026-09-09",
      hasta: "2026-09-09",
      sitioIds: ["s1"],
      busqueda: "José Pérez",
      pagina: 0,
      tamano: 50,
    });

    expect(encadenable.gte).toHaveBeenCalledWith("hora_entrada", "2026-09-09T00:00:00-06:00");
    expect(encadenable.lt).toHaveBeenCalledWith("hora_entrada", "2026-09-10T00:00:00-06:00");
    expect(encadenable.in).toHaveBeenCalledWith("sitio_id", ["s1"]);
    expect(encadenable.like).toHaveBeenCalledTimes(2);
    expect(encadenable.like).toHaveBeenCalledWith("texto_busqueda", "%jose%");
    expect(encadenable.like).toHaveBeenCalledWith("texto_busqueda", "%perez%");
  });

  it("los filtros por columna de la grilla se aplican como un .or() por columna", async () => {
    const encadenable = mockConsulta({ data: [], error: null, count: 0 });
    mocks.from.mockReturnValue(encadenable);

    await listarMovimientosPagina({
      pagina: 0,
      tamano: 50,
      filtros: {
        empresa_nombre: { filterType: "text", type: "equals", filter: "BAC" },
        gafete_numero: { filterType: "number", type: "greaterThan", filter: 3 },
      },
    });

    expect(encadenable.or).toHaveBeenCalledTimes(2);
    expect(encadenable.or).toHaveBeenCalledWith('empresa_p.eq."bac"');
    expect(encadenable.or).toHaveBeenCalledWith("gafete_numero.gt.3");
  });

  it("sin filtros no manda gte, lt, in, like ni or", async () => {
    const encadenable = mockConsulta({ data: [], error: null, count: 0 });
    mocks.from.mockReturnValue(encadenable);

    await listarMovimientosPagina({ pagina: 0, tamano: 50 });

    expect(encadenable.gte).not.toHaveBeenCalled();
    expect(encadenable.lt).not.toHaveBeenCalled();
    expect(encadenable.in).not.toHaveBeenCalled();
    expect(encadenable.like).not.toHaveBeenCalled();
    expect(encadenable.or).not.toHaveBeenCalled();
  });

  it("propaga el error como Error real y rechaza filas con forma inesperada", async () => {
    mocks.from.mockReturnValue(mockConsulta({ data: null, error: { message: "timeout" }, count: null }));
    await expect(listarMovimientosPagina({ pagina: 0, tamano: 50 })).rejects.toThrow("timeout");

    mocks.from.mockReturnValue(
      mockConsulta({ data: [filaVista({ gafete_numero: "12" })], error: null, count: 1 }),
    );
    await expect(listarMovimientosPagina({ pagina: 0, tamano: 50 })).rejects.toThrow();
  });
});

describe("listarMovimientosParaExportar", () => {
  it("pide todo el filtro en tramos de 1.000 y se detiene al recibir un tramo corto", async () => {
    const encadenable = mockConsulta({ data: [], error: null, count: 2500 });
    const tramo = (n: number) => Array.from({ length: n }, (_, i) => filaVista({ id: `f${i}` }));
    let llamada = 0;
    encadenable.then = (resolver: (valor: unknown) => void) => {
      const tamanos = [1000, 1000, 500];
      resolver({ data: tramo(tamanos[llamada++]), error: null, count: 2500 });
    };
    mocks.from.mockReturnValue(encadenable);

    const resultado = await listarMovimientosParaExportar({ busqueda: "perez" });

    expect(resultado.filas).toHaveLength(2500);
    expect(resultado.total).toBe(2500);
    expect(resultado.truncado).toBe(false);
    expect(encadenable.range).toHaveBeenNthCalledWith(1, 0, 999);
    expect(encadenable.range).toHaveBeenNthCalledWith(2, 1000, 1999);
    expect(encadenable.range).toHaveBeenNthCalledWith(3, 2000, 2999);
    // Sólo el primer tramo pide el conteo exacto: el total no cambia entre tramos.
    expect(encadenable.select).toHaveBeenNthCalledWith(1, expect.any(String), { count: "exact" });
    expect(encadenable.select).toHaveBeenNthCalledWith(2, expect.any(String), undefined);
    expect(encadenable.select).toHaveBeenNthCalledWith(3, expect.any(String), undefined);
  });

  it("marca truncado cuando el filtro tiene más filas que el máximo", async () => {
    const encadenable = mockConsulta({ data: [], error: null, count: 9999 });
    encadenable.then = (resolver: (valor: unknown) => void) =>
      resolver({
        data: Array.from({ length: 1000 }, (_, i) => filaVista({ id: `f${i}` })),
        error: null,
        count: 9999,
      });
    mocks.from.mockReturnValue(encadenable);

    const resultado = await listarMovimientosParaExportar({}, 1000);

    expect(resultado.filas).toHaveLength(1000);
    expect(resultado.total).toBe(9999);
    expect(resultado.truncado).toBe(true);
  });
});

describe("listarUnidadesOperativas", () => {
  it("devuelve id/nombre ordenados como los mandó Supabase", async () => {
    const sitios = [
      { id: "s1", nombre: "Brisas" },
      { id: "s2", nombre: "Otra unidad" },
    ];
    mocks.from.mockReturnValue(mockConsulta({ data: sitios, error: null, count: null }));

    const resultado = await listarUnidadesOperativas();

    expect(resultado).toEqual(sitios);
  });

  it("propaga el error de la consulta como Error real", async () => {
    mocks.from.mockReturnValue(mockConsulta({ data: null, error: { message: "timeout" }, count: null }));

    await expect(listarUnidadesOperativas()).rejects.toThrow("timeout");
  });
});
