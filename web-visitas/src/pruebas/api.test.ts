import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  cancelarCita,
  crearCita,
  editarCita,
  listarCitasActuales,
  listarHistorial,
  llegadasDe,
  mensajeError,
  visitantesAnteriores,
} from "../api";
import { CitaInvalida } from "../dominio";
import { hoyCostaRica } from "../fecha";

const dobles = vi.hoisted(() => ({ rpc: vi.fn(), from: vi.fn() }));
vi.mock("../lib/supabase", () => ({ supabase: dobles }));
const id = "00000000-0000-4000-8000-000000000001";
const correo = "prueba@example.invalid";
const datos = () => ({
  fecha_desde: hoyCostaRica(),
  fecha_hasta: hoyCostaRica(),
  hora_estimada: "",
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

const METODOS = ["select", "eq", "gte", "lt", "or", "order", "range", "limit", "maybeSingle", "abortSignal"] as const;
function consulta(respuesta: unknown) {
  const cadena = Object.assign(
    Promise.resolve(respuesta),
    Object.fromEntries(METODOS.map((m) => [m, vi.fn()])) as Record<(typeof METODOS)[number], ReturnType<typeof vi.fn>>,
  );
  for (const metodo of METODOS) cadena[metodo].mockReturnValue(cadena);
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
  it("manda p_hora_estimada en el payload, null si no se cargó", async () => {
    dobles.rpc.mockResolvedValue({ data: id, error: null });
    await crearCita(id, datos());
    expect(dobles.rpc).toHaveBeenCalledWith(
      "crear_cita_anfitrion",
      expect.objectContaining({ p_hora_estimada: null }),
    );
    dobles.rpc.mockResolvedValue({
      data: "00000000-0000-4000-8000-000000000003",
      error: null,
    });
    await crearCita("00000000-0000-4000-8000-000000000003", {
      ...datos(),
      hora_estimada: "14:30",
    });
    expect(dobles.rpc).toHaveBeenCalledWith(
      "crear_cita_anfitrion",
      expect.objectContaining({ p_hora_estimada: "14:30" }),
    );
  });
  it("rechaza datos inválidos antes de hacer peticiones", async () => {
    const rechazo = crearCita(id, { ...datos(), sitios: [] });
    await expect(rechazo).rejects.toBeInstanceOf(CitaInvalida);
    await expect(rechazo).rejects.toMatchObject({ errores: { sitios: "Elija al menos un lugar." } });
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
describe("edición y cancelación", () => {
  const nuevo = "00000000-0000-4000-8000-000000000009";
  it("editar manda la cita vieja y la nueva en una sola RPC y confirma el id nuevo", async () => {
    dobles.rpc.mockResolvedValue({ data: nuevo, error: null });
    await expect(editarCita(id, nuevo, datos())).resolves.toBe(nuevo);
    expect(dobles.rpc).toHaveBeenCalledWith(
      "editar_cita_anfitrion",
      expect.objectContaining({ p_id: id, p_nuevo_id: nuevo, p_visitantes: [expect.objectContaining({ cedula: "AB123" })] }),
    );
    expect(dobles.from).not.toHaveBeenCalled();
    dobles.rpc.mockResolvedValue({ data: id, error: null });
    await expect(editarCita(id, nuevo, datos())).rejects.toThrow("no corresponde");
  });
  it("editar valida antes de llamar al servidor", async () => {
    await expect(editarCita(id, nuevo, { ...datos(), visitantes: [] })).rejects.toThrow();
    await expect(editarCita("no-es-uuid", nuevo, datos())).rejects.toThrow();
    expect(dobles.rpc).not.toHaveBeenCalled();
  });
  it("cancelar usa la RPC (no escribe la tabla) y propaga el error", async () => {
    dobles.rpc.mockResolvedValue({ data: null, error: null });
    await expect(cancelarCita(id)).resolves.toBeUndefined();
    expect(dobles.rpc).toHaveBeenCalledWith("cancelar_cita_anfitrion", { p_id: id });
    expect(dobles.from).not.toHaveBeenCalled();
    dobles.rpc.mockResolvedValue({ data: null, error: { code: "P0002", message: "La visita no existe" } });
    await expect(cancelarCita(id)).rejects.toMatchObject({ code: "P0002" });
  });
});
describe("lecturas", () => {
  it("las actuales se acotan al anfitrión, vigentes y de hoy en adelante", async () => {
    const cadena = consulta({ data: [], error: null });
    await listarCitasActuales(correo);
    expect(cadena.eq).toHaveBeenCalledWith("anfitrion_correo", correo);
    expect(cadena.eq).toHaveBeenCalledWith("estado", "VIGENTE");
    expect(cadena.gte).toHaveBeenCalledWith("fecha_hasta", hoyCostaRica());
  });
  it("el historial trae canceladas o vencidas y pide una fila de más para saber si hay más", async () => {
    const cadena = consulta({ data: [], error: null });
    const resultado = await listarHistorial(correo, 2);
    expect(cadena.eq).toHaveBeenCalledWith("anfitrion_correo", correo);
    expect(cadena.or).toHaveBeenCalledWith(`estado.eq.CANCELADA,fecha_hasta.lt.${hoyCostaRica()}`);
    expect(cadena.range).toHaveBeenCalledWith(40, 60);
    expect(resultado).toEqual({ citas: [], hayMas: false });
  });
  it("llegadas: sin citas no consulta; con citas valida la respuesta", async () => {
    await expect(llegadasDe([])).resolves.toEqual([]);
    expect(dobles.rpc).not.toHaveBeenCalled();
    dobles.rpc.mockResolvedValue({ data: [{ cita_visitante_id: "x" }], error: null });
    await expect(llegadasDe([id])).rejects.toThrow();
    expect(dobles.rpc).toHaveBeenCalledWith("estado_visitantes_de_mis_citas", { p_citas: [id] });
  });
  it("visitantes anteriores: búsqueda vacía va como null", async () => {
    const cadena = consulta({ data: [], error: null });
    dobles.rpc.mockReturnValue(cadena);
    await visitantesAnteriores("   ");
    expect(dobles.rpc).toHaveBeenCalledWith("visitantes_anteriores", { p_busqueda: null });
    await visitantesAnteriores(" ana ");
    expect(dobles.rpc).toHaveBeenCalledWith("visitantes_anteriores", { p_busqueda: "ana" });
  });
  it("no muestra mensajes internos del servidor pero sí los de las reglas", () => {
    expect(mensajeError({ message: "SQL privado con datos de visitante" })).not.toContain("SQL");
    expect(mensajeError({ code: "P0001", message: "Alguien de esta visita ya entró" })).toBe("Alguien de esta visita ya entró");
  });
});
