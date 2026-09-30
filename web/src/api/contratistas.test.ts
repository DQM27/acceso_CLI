import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { crearContratista, crearEmpresa, listarContratistas, listarEmpresas } from "./contratistas";

/**
 * Construye un mock encadenable de la query de supabase-js
 * (`.from().select().order().range().returns()`) que resuelve al
 * `awaitearlo` -- mismo shape real que usa el cliente, sin levantar un
 * servidor de verdad.
 */
function mockConsulta(resultado: { data: unknown; error: unknown; count: number | null }) {
  const encadenable: Record<string, unknown> = {
    select: vi.fn(() => encadenable),
    order: vi.fn(() => encadenable),
    eq: vi.fn(() => encadenable),
    range: vi.fn(() => encadenable),
    returns: vi.fn(() => encadenable),
    then: (resolver: (valor: typeof resultado) => void) => resolver(resultado),
  };
  return encadenable;
}

const mocks = vi.hoisted(() => ({ from: vi.fn(), rpc: vi.fn() }));
vi.mock("../lib/supabase", () => ({ supabase: { from: mocks.from, rpc: mocks.rpc } }));

beforeEach(() => {
  vi.clearAllMocks();
});
afterEach(() => {
  vi.restoreAllMocks();
});

function filaCompleta(sobrescribir: Record<string, unknown> = {}) {
  return {
    id: "1",
    identificacion: "1-2345-6789",
    nombre: "Alguien",
    empresa_nombre: "Constructora X",
    tipo_ingreso: "PRAIND",
    fecha_vencimiento_praind: "2027-01-01",
    es_personal_ruta: false,
    activo: true,
    empresa_activa: true,
    requiere_praind: true,
    dias_para_vencer: 90,
    estado_praind: "VIGENTE",
    estado_acceso: "PERMITIDO",
    adentro_sitio_nombre: null,
    adentro_desde: null,
    ...sobrescribir,
  };
}

describe("listarContratistas", () => {
  it("lee la vista panel_contratistas_estado con el estado calculado en el servidor", async () => {
    const encadenable = mockConsulta({
      data: [filaCompleta({ estado_acceso: "PRAIND_VENCIDO", estado_praind: "VENCIDA", dias_para_vencer: -3 })],
      error: null,
      count: 1,
    });
    mocks.from.mockReturnValue(encadenable);

    const resultado = await listarContratistas();

    expect(mocks.from).toHaveBeenCalledWith("panel_contratistas_estado");
    expect(resultado.filas[0].estado_acceso).toBe("PRAIND_VENCIDO");
    expect(resultado.filas[0].dias_para_vencer).toBe(-3);
  });

  it("rechaza un estado que la vista no define", async () => {
    mocks.from.mockReturnValue(
      mockConsulta({ data: [filaCompleta({ estado_acceso: "INVENTADO" })], error: null, count: 1 }),
    );
    await expect(listarContratistas()).rejects.toThrow();
  });

  it("truncado en false cuando el conteo real coincide con lo que vino", async () => {
    const filas = [filaCompleta()];
    mocks.from.mockReturnValue(mockConsulta({ data: filas, error: null, count: 1 }));

    const resultado = await listarContratistas();

    expect(resultado.truncado).toBe(false);
    expect(resultado.filas).toEqual(filas);
  });

  it("truncado en true cuando el conteo real es mayor que las filas devueltas (tope alcanzado)", async () => {
    const filas = Array.from({ length: 3 }, (_, i) => filaCompleta({ id: String(i), nombre: `Fila ${i}` }));
    // El conteo real (lo que devuelve Postgres con count:'exact') es mayor
    // que lo que vino en `data` -- exactamente lo que pasa cuando `.range()`
    // corta antes de llegar al final.
    mocks.from.mockReturnValue(mockConsulta({ data: filas, error: null, count: 50_000 }));

    const resultado = await listarContratistas();

    expect(resultado.truncado).toBe(true);
    expect(resultado.filas).toHaveLength(3);
  });

  it("propaga el error de la consulta como Error real", async () => {
    mocks.from.mockReturnValue(
      mockConsulta({ data: null, error: { message: "RLS denegó el acceso" }, count: null }),
    );

    await expect(listarContratistas()).rejects.toThrow("RLS denegó el acceso");
  });

  it("lanza un error de validación si Supabase devuelve una fila con forma inesperada", async () => {
    // Simula un cambio de contrato del backend (ej. `activo` deja de ser
    // boolean) -- sin la validación zod, esto pasaría en silencio.
    const filas = [filaCompleta({ activo: "sí" })];
    mocks.from.mockReturnValue(mockConsulta({ data: filas, error: null, count: 1 }));

    await expect(listarContratistas()).rejects.toThrow();
  });
});

describe("listarEmpresas", () => {
  it("pide solo las empresas activas, ordenadas por nombre", async () => {
    const consulta = mockConsulta({ data: [{ id: "e1", nombre: "BAC" }], error: null, count: null });
    mocks.from.mockReturnValue(consulta);

    const empresas = await listarEmpresas();

    expect(mocks.from).toHaveBeenCalledWith("empresas");
    expect(consulta.eq).toHaveBeenCalledWith("activa", true);
    expect(consulta.order).toHaveBeenCalledWith("nombre");
    expect(empresas).toEqual([{ id: "e1", nombre: "BAC" }]);
  });

  it("propaga el error como Error real", async () => {
    mocks.from.mockReturnValue(mockConsulta({ data: null, error: { message: "sin permiso" }, count: null }));
    await expect(listarEmpresas()).rejects.toThrow("sin permiso");
  });
});

describe("crearEmpresa", () => {
  it("llama a panel_crear_empresa y devuelve la empresa", async () => {
    mocks.rpc.mockResolvedValue({ data: { id: "e2", nombre: "NUEVA", activa: true }, error: null });

    const empresa = await crearEmpresa("nueva");

    expect(mocks.rpc).toHaveBeenCalledWith("panel_crear_empresa", { p_nombre: "nueva" });
    expect(empresa).toEqual({ id: "e2", nombre: "NUEVA" });
  });

  it("propaga el mensaje de la base", async () => {
    mocks.rpc.mockResolvedValue({ data: null, error: { message: "El nombre de la empresa es obligatorio" } });
    await expect(crearEmpresa("")).rejects.toThrow("El nombre de la empresa es obligatorio");
  });
});

describe("crearContratista", () => {
  const datos = {
    cedula: "112340567",
    nombre: "Ana",
    empresa_id: "e1",
    tipo_ingreso: "SWAT" as const,
    fecha_vencimiento_praind: null,
    con_acceso: false,
  };

  it("llama a panel_crear_contratista con los parametros de la base y devuelve la fila", async () => {
    mocks.rpc.mockResolvedValue({ data: filaCompleta({ activo: false, nombre: "ANA" }), error: null });

    const creado = await crearContratista(datos);

    expect(mocks.rpc).toHaveBeenCalledWith("panel_crear_contratista", {
      p_cedula: "112340567",
      p_nombre: "Ana",
      p_empresa_id: "e1",
      p_tipo_ingreso: "SWAT",
      p_fecha_vencimiento_praind: null,
      p_con_acceso: false,
    });
    expect(creado.activo).toBe(false);
    expect(creado.nombre).toBe("ANA");
  });

  it("propaga el mensaje en español que devuelve la base", async () => {
    mocks.rpc.mockResolvedValue({ data: null, error: { message: "La cédula del contratista ya existe" } });
    await expect(crearContratista(datos)).rejects.toThrow("La cédula del contratista ya existe");
  });

  it("rechaza una respuesta con forma inesperada", async () => {
    mocks.rpc.mockResolvedValue({ data: filaCompleta({ activo: "sí" }), error: null });
    await expect(crearContratista(datos)).rejects.toThrow();
  });
});
