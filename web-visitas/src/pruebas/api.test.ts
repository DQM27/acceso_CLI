import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  cancelarVisita,
  crearVisitas,
  duplicarVisita,
  mensajeError,
  responderSolicitud,
} from "../api";
import { hoyCostaRica } from "../fecha";

const dobles = vi.hoisted(() => ({ rpc: vi.fn(), from: vi.fn() }));
vi.mock("../lib/supabase", () => ({ supabase: dobles }));

const grupoId = "00000000-0000-4000-8000-000000000001";
const sitioId = "00000000-0000-4000-8000-000000000002";
const visitaId = "00000000-0000-4000-8000-000000000003";
const invitadoId = "00000000-0000-4000-8000-000000000004";

const datos = () => ({
  fecha_desde: hoyCostaRica(),
  fecha_hasta: hoyCostaRica(),
  hora_desde: "08:00",
  hora_hasta: "17:00",
  tipo_visita: "",
  motivo: "",
  requiere_escolta: false,
  sitios: [sitioId],
  invitados: [
    {
      tipo_documento: "CEDULA" as const,
      numero_documento: "AB-123",
      nombre: "Persona de prueba",
      empresa: "",
      telefono: "",
      correo: "",
      placa_vehiculo: "",
    },
  ],
});

beforeEach(() => vi.resetAllMocks());

describe("crearVisitas", () => {
  it("manda una sola RPC con los invitados normalizados", async () => {
    dobles.rpc.mockResolvedValue({ data: [visitaId], error: null });
    expect(await crearVisitas(grupoId, datos())).toEqual([visitaId]);
    expect(dobles.rpc).toHaveBeenCalledWith(
      "crear_visitas",
      expect.objectContaining({
        p_grupo_id: grupoId,
        p_sitios: [sitioId],
        p_invitados: [expect.objectContaining({ numero_documento: "AB123" })],
      }),
    );
  });
  it("rechaza datos inválidos antes de llamar a la RPC", async () => {
    await expect(crearVisitas(grupoId, { ...datos(), sitios: [] })).rejects.toThrow();
    expect(dobles.rpc).not.toHaveBeenCalled();
  });
  it("propaga el error de la RPC sin escrituras parciales del lado del cliente", async () => {
    dobles.rpc.mockResolvedValue({ data: null, error: { code: "PGRST202" } });
    await expect(crearVisitas(grupoId, datos())).rejects.toEqual({ code: "PGRST202" });
    expect(dobles.from).not.toHaveBeenCalled();
  });
});

describe("cancelarVisita / duplicarVisita / responderSolicitud", () => {
  it("cancelarVisita llama a la RPC con el id de la visita", async () => {
    dobles.rpc.mockResolvedValue({ error: null });
    await cancelarVisita(visitaId);
    expect(dobles.rpc).toHaveBeenCalledWith("cancelar_visita", { p_visita_id: visitaId });
  });
  it("duplicarVisita valida las fechas antes de llamar a la RPC", async () => {
    await expect(duplicarVisita(visitaId, grupoId, "fecha-invalida", "2026-09-10")).rejects.toThrow();
    expect(dobles.rpc).not.toHaveBeenCalled();
  });
  it("responderSolicitud manda aprobar/rechazar con el motivo", async () => {
    dobles.rpc.mockResolvedValue({ error: null });
    await responderSolicitud(invitadoId, false, "no corresponde");
    expect(dobles.rpc).toHaveBeenCalledWith("responder_solicitud", {
      p_invitado_id: invitadoId,
      p_aprobar: false,
      p_motivo_rechazo: "no corresponde",
    });
  });
});

describe("mensajeError", () => {
  it("no muestra mensajes internos del servidor", () => {
    expect(
      mensajeError({ message: "SQL privado con datos de visitante" }),
    ).not.toContain("SQL");
  });
  it("da un mensaje claro para permisos y solicitudes repetidas", () => {
    expect(mensajeError({ code: "42501" })).toContain("permiso");
    expect(mensajeError({ code: "23505" })).toContain("existe");
  });
});
