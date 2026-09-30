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
  it("lee la vista panel_movimientos sin pedir conteo exacto y da el total al llegar a la última página", async () => {
    const encadenable = mockConsulta({ data: [filaVista()], error: null, count: null });
    mocks.from.mockReturnValue(encadenable);

    const resultado = await listarMovimientosPagina({ pagina: 2, tamano: 100 });

    expect(mocks.from).toHaveBeenCalledWith("panel_movimientos");
    // El conteo exacto recorre todo el filtro (segundos con volumen alto).
    expect(encadenable.select).toHaveBeenCalledWith(expect.any(String));
    // Página corta: es la última, así que el total se conoce sin contar.
    expect(resultado.total).toBe(201);
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

    // La empresa suma su prefiltro contra el índice de texto_busqueda.
    expect(encadenable.or).toHaveBeenCalledTimes(3);
    expect(encadenable.or).toHaveBeenCalledWith('empresa_p.eq."bac"');
    expect(encadenable.or).toHaveBeenCalledWith('texto_busqueda.like."*bac*"');
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

  it("deja el total sin definir mientras la página venga llena", async () => {
    const llena = Array.from({ length: 50 }, (_, i) => filaVista({ id: `f${i}` }));
    mocks.from.mockReturnValue(mockConsulta({ data: llena, error: null, count: null }));

    const resultado = await listarMovimientosPagina({ pagina: 0, tamano: 50 });

    expect(resultado.filas).toHaveLength(50);
    expect(resultado.total).toBeUndefined();
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
    expect(resultado.truncado).toBe(false);
    expect(encadenable.range).toHaveBeenNthCalledWith(1, 0, 999);
    expect(encadenable.range).toHaveBeenNthCalledWith(2, 1000, 1999);
    expect(encadenable.range).toHaveBeenNthCalledWith(3, 2000, 2999);
    expect(encadenable.range).toHaveBeenCalledTimes(3);
  });

  it("al llegar al máximo pide una sola fila más para saber si quedó algo afuera", async () => {
    const encadenable = mockConsulta({ data: [], error: null, count: null });
    const tramos = [1000, 1];
    let llamada = 0;
    encadenable.then = (resolver: (valor: unknown) => void) =>
      resolver({
        data: Array.from({ length: tramos[llamada++] }, (_, i) => filaVista({ id: `f${i}` })),
        error: null,
        count: null,
      });
    mocks.from.mockReturnValue(encadenable);

    const resultado = await listarMovimientosParaExportar({}, 1000);

    expect(resultado.filas).toHaveLength(1000);
    expect(resultado.truncado).toBe(true);
    expect(encadenable.range).toHaveBeenNthCalledWith(2, 1000, 1000);
  });

  it("no marca truncado si el filtro tiene exactamente el máximo", async () => {
    const encadenable = mockConsulta({ data: [], error: null, count: null });
    const tramos = [1000, 0];
    let llamada = 0;
    encadenable.then = (resolver: (valor: unknown) => void) =>
      resolver({
        data: Array.from({ length: tramos[llamada++] }, (_, i) => filaVista({ id: `f${i}` })),
        error: null,
        count: null,
      });
    mocks.from.mockReturnValue(encadenable);

    const resultado = await listarMovimientosParaExportar({}, 1000);

    expect(resultado.filas).toHaveLength(1000);
    expect(resultado.truncado).toBe(false);
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
