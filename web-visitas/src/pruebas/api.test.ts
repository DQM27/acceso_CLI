import { beforeEach, describe, expect, it, vi } from "vitest";
import { crearCita, cancelarCita, listarCitas, mensajeError } from "../api";
import { hoyCostaRica } from "../fecha";

const dobles = vi.hoisted(() => ({ rpc: vi.fn(), from: vi.fn() }));
vi.mock("../lib/supabase", () => ({ supabase: dobles }));
const id = "00000000-0000-4000-8000-000000000001";
const correo = "prueba@example.invalid";
const datos = () => ({
  fecha_desde: hoyCostaRica(),
  fecha_hasta: hoyCostaRica(),
  motivo: "",
  sitios: [id],
  visitantes: [
    {
      nombre: "Persona de prueba",
      cedula: "AB-123",
      empresa: "",
      placa_vehiculo: "",
    },
  ],
});

function consulta(respuesta: unknown) {
  const resultado = Promise.resolve(respuesta);
  const cadena = Object.assign(resultado, {
    select: vi.fn(),
    update: vi.fn(),
    eq: vi.fn(),
    gte: vi.fn(),
    lt: vi.fn(),
    order: vi.fn(),
    range: vi.fn(),
    single: vi.fn(),
  });
  for (const metodo of [
    cadena.select,
    cadena.update,
    cadena.eq,
    cadena.gte,
    cadena.lt,
    cadena.order,
    cadena.range,
    cadena.single,
  ])
    metodo.mockReturnValue(cadena);
  dobles.from.mockReturnValue(cadena);
  return cadena;
}
beforeEach(() => vi.resetAllMocks());
describe("guardado atómico", () => {
  it("manda una sola RPC, normaliza y no permite elegir propietario", async () => {
    dobles.rpc.mockResolvedValue({ data: id, error: null });
    expect(await crearCita(id, datos())).toBe(id);
    expect(dobles.rpc).toHaveBeenCalledWith(
      "crear_cita_anfitrion",
      expect.objectContaining({
        p_id: id,
        p_visitantes: [expect.objectContaining({ cedula: "AB123" })],
      }),
    );
    const llamada = dobles.rpc.mock.calls[0];
    if (!llamada) throw new Error("crear_cita_anfitrion no fue llamado");
    expect(llamada[1]).not.toHaveProperty("anfitrion_correo");
    expect(dobles.from).not.toHaveBeenCalled();
  });
  it("rechaza datos inválidos antes de hacer peticiones", async () => {
    await expect(crearCita(id, { ...datos(), sitios: [] })).rejects.toThrow();
    expect(dobles.rpc).not.toHaveBeenCalled();
  });
  it("si no existe la RPC no hace escrituras parciales", async () => {
    dobles.rpc.mockResolvedValue({ data: null, error: { code: "PGRST202" } });
    await expect(crearCita(id, datos())).rejects.toEqual({ code: "PGRST202" });
    expect(dobles.from).not.toHaveBeenCalled();
  });
  it("permite recuperar una solicitud incierta después de medianoche", async () => {
    dobles.rpc.mockResolvedValue({ data: id, error: null });
    const anterior = {
      ...datos(),
      fecha_desde: "2026-01-01",
      fecha_hasta: "2026-01-01",
    };
    await expect(crearCita(id, anterior, "2026-01-01")).resolves.toBe(id);
    await expect(crearCita(id, anterior, "2026-01-02")).rejects.toThrow();
  });
  it("un reintento conserva la misma clave y comprueba el UUID devuelto", async () => {
    dobles.rpc
      .mockResolvedValueOnce({ data: null, error: { code: "timeout" } })
      .mockResolvedValue({ data: id, error: null });
    await expect(crearCita(id, datos())).rejects.toBeDefined();
    await crearCita(id, datos());
    expect(dobles.rpc.mock.calls[0]).toEqual(dobles.rpc.mock.calls[1]);
    dobles.rpc.mockResolvedValue({
      data: "00000000-0000-4000-8000-000000000002",
      error: null,
    });
    await expect(crearCita(id, datos())).rejects.toThrow("no corresponde");
  });
});
describe("lectura y cancelación", () => {
  it("acota propietario, estado y paginación en la consulta", async () => {
    const cadena = consulta({ data: [], error: null });
    await listarCitas(correo, "VENCIDA", 2);
    expect(cadena.eq).toHaveBeenCalledWith("anfitrion_correo", correo);
    expect(cadena.eq).toHaveBeenCalledWith("estado", "VIGENTE");
    expect(cadena.lt).toHaveBeenCalledWith("fecha_hasta", hoyCostaRica());
    expect(cadena.range).toHaveBeenCalledWith(24, 36);
  });
  it("solo confirma cancelación si el servidor devuelve la fila", async () => {
    const cadena = consulta({ data: null, error: null });
    await expect(cancelarCita(id, correo)).rejects.toThrow();
    expect(cadena.eq).toHaveBeenCalledWith("anfitrion_correo", correo);
    expect(cadena.eq).toHaveBeenCalledWith("estado", "VIGENTE");
    expect(cadena.update).toHaveBeenCalledWith({ estado: "CANCELADA" });
    consulta({ data: { id }, error: null });
    await expect(cancelarCita(id, correo)).resolves.toBeUndefined();
  });
  it("no muestra mensajes internos del servidor", () => {
    expect(
      mensajeError({ message: "SQL privado con datos de visitante" }),
    ).not.toContain("SQL");
  });
});
