import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  listarMovimientosPagina,
  listarMovimientosParaExportar,
  listarUnidadesOperativas,
} from "./historial";

function mockConsulta(resultado: {
  data: unknown;
  error: unknown;
  count: number | null;
}) {
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

const mocks = vi.hoisted(() => ({ from: vi.fn(), rpc: vi.fn() }));
vi.mock("../lib/supabase", () => ({
  supabase: { from: mocks.from, rpc: mocks.rpc },
}));

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(() => {
  vi.restoreAllMocks();
});

/** Los parámetros de la última llamada a `panel_historial_contratistas`. */
function parametros(llamada = -1): Record<string, unknown> {
  const llamadas = mocks.rpc.mock.calls;
  const [nombre, valores] = llamadas.at(llamada) as [
    string,
    Record<string, unknown>,
  ];
  expect(nombre).toBe("panel_historial_contratistas");
  return valores;
}

function filas(cantidad: number, prefijo = "f") {
  return Array.from({ length: cantidad }, (_, i) =>
    filaVista({
      id: `${prefijo}${i}`,
      hora_entrada: `2026-08-20T14:${String(59 - (i % 60)).padStart(2, "0")}:00Z`,
    }),
  );
}

/** Responde cada llamada con el siguiente tamaño de la lista. */
function responderEnTramos(tamanos: number[]) {
  const encadenable = mockConsulta({ data: [], error: null, count: null });
  let llamada = 0;
  encadenable.then = (resolver: (valor: unknown) => void) => {
    const n = tamanos[llamada];
    resolver({ data: filas(n, `t${llamada}-`), error: null, count: null });
    llamada += 1;
  };
  mocks.rpc.mockReturnValue(encadenable);
  return encadenable;
}

describe("listarMovimientosPagina", () => {
  it("pide la página a panel_historial_contratistas y da el total al llegar a la última", async () => {
    mocks.rpc.mockReturnValue(
      mockConsulta({ data: [filaVista()], error: null, count: null }),
    );

    const resultado = await listarMovimientosPagina({ pagina: 2, tamano: 100 });

    expect(mocks.from).not.toHaveBeenCalled();
    expect(parametros()).toMatchObject({
      p_limite: 100,
      // Página nunca vista: salta las anteriores.
      p_desplazamiento: 200,
      p_cursor_hora: null,
      p_cursor_id: null,
      p_orden: "hora_entrada",
      p_descendente: true,
    });
    // Página corta: es la última, así que el total se conoce sin contar.
    expect(resultado.total).toBe(201);
    expect(resultado.filas[0].sitio_nombre).toBe("Brisas");
  });

  it("respeta el orden pedido", async () => {
    mocks.rpc.mockReturnValue(
      mockConsulta({ data: [], error: null, count: 0 }),
    );

    await listarMovimientosPagina({
      pagina: 0,
      tamano: 50,
      orden: { campo: "empresa_nombre", descendente: false },
    });

    expect(parametros()).toMatchObject({
      p_orden: "empresa_nombre",
      p_descendente: false,
    });
  });

  it("manda fechas (día de Costa Rica), unidades y la búsqueda recortada; sin búsqueda, null", async () => {
    mocks.rpc.mockReturnValue(
      mockConsulta({ data: [], error: null, count: 0 }),
    );

    await listarMovimientosPagina({
      desde: "2026-09-09",
      hasta: "2026-09-09",
      sitioIds: ["s1"],
      busqueda: "  José Pérez ",
      pagina: 0,
      tamano: 50,
    });
    expect(parametros()).toMatchObject({
      p_desde: "2026-09-09T00:00:00-06:00",
      p_hasta: "2026-09-10T00:00:00-06:00",
      p_sitio_ids: ["s1"],
      p_busqueda: "José Pérez",
    });

    await listarMovimientosPagina({ busqueda: "   ", pagina: 0, tamano: 51 });
    expect(parametros()).toMatchObject({
      p_desde: null,
      p_hasta: null,
      p_sitio_ids: null,
      p_busqueda: null,
    });
  });

  it("los filtros de la grilla viajan como datos, uno por columna", async () => {
    mocks.rpc.mockReturnValue(
      mockConsulta({ data: [], error: null, count: 0 }),
    );

    await listarMovimientosPagina({
      pagina: 0,
      tamano: 52,
      filtros: {
        empresa_nombre: { filterType: "text", type: "equals", filter: "BAC" },
        gafete_numero: { filterType: "number", type: "greaterThan", filter: 3 },
      },
    });

    expect(parametros().p_filtros).toEqual([
      {
        columna: "empresa_p",
        union: "and",
        condiciones: [{ op: "igual", valor: "BAC" }],
      },
      {
        columna: "gafete_numero",
        union: "and",
        condiciones: [{ op: "mayor", valor: 3 }],
      },
    ]);
  });

  it("la página siguiente sale por cursor desde la última fila de la anterior", async () => {
    responderEnTramos([2, 2, 2]);
    const consulta = { desde: "2026-01-02", tamano: 2 };

    const primera = await listarMovimientosPagina({ ...consulta, pagina: 0 });
    await listarMovimientosPagina({ ...consulta, pagina: 1 });

    const ultima = primera.filas[primera.filas.length - 1];
    expect(parametros()).toMatchObject({
      p_desplazamiento: 0,
      p_cursor_hora: ultima.hora_entrada,
      p_cursor_id: ultima.id,
    });

    // A una página nunca vista se salta como antes.
    await listarMovimientosPagina({ ...consulta, pagina: 5 });
    expect(parametros()).toMatchObject({
      p_desplazamiento: 10,
      p_cursor_id: null,
    });
  });

  it("con otro orden que no sea por hora no usa cursor", async () => {
    responderEnTramos([2, 2]);
    const consulta = {
      desde: "2026-01-03",
      tamano: 2,
      orden: { campo: "empresa_nombre" as const, descendente: false },
    };

    await listarMovimientosPagina({ ...consulta, pagina: 0 });
    await listarMovimientosPagina({ ...consulta, pagina: 1 });

    expect(parametros()).toMatchObject({
      p_desplazamiento: 2,
      p_cursor_id: null,
    });
  });

  it("propaga el error como Error real y rechaza filas con forma inesperada", async () => {
    mocks.rpc.mockReturnValue(
      mockConsulta({ data: null, error: { message: "timeout" }, count: null }),
    );
    await expect(
      listarMovimientosPagina({ pagina: 0, tamano: 50 }),
    ).rejects.toThrow("timeout");

    mocks.rpc.mockReturnValue(
      mockConsulta({
        data: [filaVista({ gafete_numero: "12" })],
        error: null,
        count: 1,
      }),
    );
    await expect(
      listarMovimientosPagina({ pagina: 0, tamano: 50 }),
    ).rejects.toThrow();
  });

  it("deja el total sin definir mientras la página venga llena", async () => {
    mocks.rpc.mockReturnValue(
      mockConsulta({ data: filas(50), error: null, count: null }),
    );

    const resultado = await listarMovimientosPagina({ pagina: 0, tamano: 50 });

    expect(resultado.filas).toHaveLength(50);
    expect(resultado.total).toBeUndefined();
  });
});

describe("listarMovimientosParaExportar", () => {
  it("pide todo el filtro en tramos de 1.000, cada uno por cursor, y se detiene en un tramo corto", async () => {
    responderEnTramos([1000, 1000, 500]);

    const resultado = await listarMovimientosParaExportar({
      busqueda: "perez",
    });

    expect(mocks.rpc).toHaveBeenCalledTimes(3);
    expect(resultado.filas).toHaveLength(2500);
    expect(resultado.truncado).toBe(false);
    expect(parametros(0)).toMatchObject({
      p_limite: 1000,
      p_desplazamiento: 0,
      p_cursor_id: null,
    });
    expect(parametros(1)).toMatchObject({
      p_limite: 1000,
      p_desplazamiento: 0,
      p_cursor_id: resultado.filas[999].id,
      p_cursor_hora: resultado.filas[999].hora_entrada,
    });
    expect(parametros(2)).toMatchObject({
      p_cursor_id: resultado.filas[1999].id,
    });
  });

  it("con otro orden que no sea por hora salta filas", async () => {
    responderEnTramos([1000, 3]);

    await listarMovimientosParaExportar({
      orden: { campo: "empresa_nombre", descendente: false },
    });

    expect(parametros(1)).toMatchObject({
      p_desplazamiento: 1000,
      p_cursor_id: null,
    });
  });

  it("al llegar al máximo pide una sola fila más para saber si quedó algo afuera", async () => {
    responderEnTramos([1000, 1]);

    const resultado = await listarMovimientosParaExportar({}, 1000);

    expect(resultado.filas).toHaveLength(1000);
    expect(resultado.truncado).toBe(true);
    expect(parametros(1)).toMatchObject({
      p_limite: 1,
      p_cursor_id: resultado.filas[999].id,
    });
  });

  it("no marca truncado si el filtro tiene exactamente el máximo", async () => {
    responderEnTramos([1000, 0]);

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
    mocks.from.mockReturnValue(
      mockConsulta({ data: sitios, error: null, count: null }),
    );

    const resultado = await listarUnidadesOperativas();

    expect(resultado).toEqual(sitios);
  });

  it("propaga el error de la consulta como Error real", async () => {
    mocks.from.mockReturnValue(
      mockConsulta({ data: null, error: { message: "timeout" }, count: null }),
    );

    await expect(listarUnidadesOperativas()).rejects.toThrow("timeout");
  });
});
