import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { campoOrdenClase, listarClaseParaExportar, listarPaginaClase } from "./historialClases";
import { expresionBusqueda } from "./historialFiltros";

function mockConsulta(resultado: { data: unknown; error: unknown }) {
  const encadenable: Record<string, unknown> = {
    select: vi.fn(() => encadenable),
    order: vi.fn(() => encadenable),
    range: vi.fn(() => encadenable),
    gte: vi.fn(() => encadenable),
    lt: vi.fn(() => encadenable),
    in: vi.fn(() => encadenable),
    or: vi.fn(() => encadenable),
    then: (resolver: (valor: typeof resultado) => void) => resolver(resultado),
  };
  return encadenable;
}

function fila(sobrescribir: Record<string, unknown> = {}) {
  return {
    id: "p1",
    sitio_id: "s1",
    sitio_nombre: "Brisas",
    identificacion: "111222333",
    nombre: "JOSÉ PÉREZ",
    detalle: "MAIKA",
    placa: null,
    gafete_numero: 7,
    hora_entrada: "2026-10-20T14:30:00Z",
    hora_salida: null,
    usuario_entrada_nombre: "OP",
    usuario_salida_nombre: null,
    dispositivo_entrada_tipo: "mobile",
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

describe("listarPaginaClase", () => {
  it("lee la vista de la clase con la forma común, ordena por entrada y acota fecha y unidades", async () => {
    const encadenable = mockConsulta({ data: [fila()], error: null });
    mocks.from.mockReturnValue(encadenable);

    const pagina = await listarPaginaClase("proveedores", { desde: "2026-10-01", sitioIds: ["s1"] }, 0, 100);

    expect(mocks.from).toHaveBeenCalledWith("panel_movimientos_proveedor");
    expect(String((encadenable.select as ReturnType<typeof vi.fn>).mock.calls[0][0])).toContain(
      "identificacion:cedula, detalle:empresa_nombre",
    );
    expect(encadenable.order).toHaveBeenCalledWith("hora_entrada", { ascending: false, nullsFirst: false });
    expect(encadenable.range).toHaveBeenCalledWith(0, 99);
    expect(encadenable.gte).toHaveBeenCalledWith("hora_entrada", expect.any(String));
    expect(encadenable.in).toHaveBeenCalledWith("sitio_id", ["s1"]);
    // Una sola fila: es la última página.
    expect(pagina).toEqual({ filas: [fila()], total: 1 });
  });

  it("KOF pide el código de empleado y no tiene empresa ni placa", async () => {
    const encadenable = mockConsulta({
      data: [fila({ identificacion: "EMP-1", detalle: undefined, placa: undefined })],
      error: null,
    });
    mocks.from.mockReturnValue(encadenable);

    const { filas } = await listarPaginaClase("kof", {}, 0, 100);

    expect(mocks.from).toHaveBeenCalledWith("panel_movimientos_kof");
    const columnas = String((encadenable.select as ReturnType<typeof vi.fn>).mock.calls[0][0]);
    expect(columnas).toContain("identificacion:codigo_empleado");
    expect(columnas).not.toContain("detalle");
    expect(filas[0].identificacion).toBe("EMP-1");
  });

  it("el orden de la grilla se traduce a la columna de la vista; el buscador y los filtros van con .or()", async () => {
    const encadenable = mockConsulta({ data: [], error: null });
    mocks.from.mockReturnValue(encadenable);

    await listarPaginaClase(
      "correo",
      {
        busqueda: "Luis",
        orden: { colId: "detalle", descendente: false },
        filtros: { placa: { filterType: "text", type: "contains", filter: "BPH" } },
      },
      0,
      100,
    );

    expect(encadenable.order).toHaveBeenCalledWith("motivo", { ascending: true, nullsFirst: false });
    expect(encadenable.or).toHaveBeenCalledWith('or(cedula_p.ilike."*luis*",nombre_p.ilike."*luis*")');
    expect(encadenable.or).toHaveBeenCalledWith('placa_p.ilike."*bph*"');
  });

  it("propaga el error de Supabase", async () => {
    mocks.from.mockReturnValue(mockConsulta({ data: null, error: { message: "sin permiso" } }));
    await expect(listarPaginaClase("kof", {}, 0, 100)).rejects.toThrow("sin permiso");
  });
});

describe("listarClaseParaExportar", () => {
  it("pide en tramos hasta el máximo y avisa si quedó algo afuera", async () => {
    mocks.from.mockImplementation(() => mockConsulta({ data: [fila(), fila({ id: "p2" })], error: null }));

    const resultado = await listarClaseParaExportar("proveedores", {}, 2);

    expect(resultado.filas).toHaveLength(2);
    expect(resultado.truncado).toBe(true);
  });
});

describe("campoOrdenClase", () => {
  it("fecha y hora ordenan por el mismo instante; lo desconocido no ordena en el servidor", () => {
    expect(campoOrdenClase("kof", "fecha_entrada")).toBe("hora_entrada");
    expect(campoOrdenClase("kof", "hora_salida")).toBe("hora_salida");
    expect(campoOrdenClase("kof", "identificacion")).toBe("codigo_empleado");
    expect(campoOrdenClase("proveedores", "no_existe")).toBeNull();
  });
});

describe("expresionBusqueda", () => {
  it("cédula con guiones también sin ellos, y nombre por palabras en cualquier orden", () => {
    expect(expresionBusqueda("1-1111", "cedula_p", "nombre_p")).toBe(
      'or(cedula_p.ilike."*1-1111*",cedula_p.ilike."*11111*",nombre_p.ilike."*1-1111*")',
    );
    expect(expresionBusqueda("Pérez José", "cedula_p", "nombre_p")).toBe(
      'or(cedula_p.ilike."*perez jose*",cedula_p.ilike."*perezjose*",and(nombre_p.ilike."*perez*",nombre_p.ilike."*jose*"))',
    );
    expect(expresionBusqueda("   ", "cedula_p", "nombre_p")).toBeNull();
  });
});
