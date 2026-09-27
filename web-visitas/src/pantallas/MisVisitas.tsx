import { useEffect, useMemo } from "react";
import { useNavigate } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CalendarPlus, CalendarX2, Check, ChevronRight, MapPin, Users, X } from "lucide-react";
import { toast } from "sonner";
import { supabase } from "../lib/supabase";
import { listarMisVisitas, mensajeError, responderSolicitud } from "../api";
import { agruparVisitas, estadoVisita } from "../dominio";
import type { VisitaAgrupada } from "../dominio";
import { fechaLegible, horaCostaRicaDe, horaLegible, hoyCostaRica } from "../fecha";
import { Cargando } from "../componentes/Comunes";

const CLAVE_MIS_VISITAS = ["mis-visitas"];

export default function MisVisitas() {
  const navegar = useNavigate();
  const clienteConsultas = useQueryClient();

  const { data, isPending, isError, error } = useQuery({
    queryKey: CLAVE_MIS_VISITAS,
    queryFn: ({ signal }) => listarMisVisitas(signal),
    staleTime: 15_000,
  });

  // Realtime: sólo invalida y vuelve a pedir -- nunca reconstruye el
  // estado a mano a partir del payload del cambio (ver 3.7, "Realtime
  // sólo invalida la consulta afectada").
  useEffect(() => {
    const canal = supabase
      .channel("mis-visitas")
      .on("postgres_changes", { event: "*", schema: "public", table: "visitas" }, () =>
        clienteConsultas.invalidateQueries({ queryKey: CLAVE_MIS_VISITAS }),
      )
      .on("postgres_changes", { event: "*", schema: "public", table: "visita_invitados" }, () =>
        clienteConsultas.invalidateQueries({ queryKey: CLAVE_MIS_VISITAS }),
      )
      .on("postgres_changes", { event: "*", schema: "public", table: "visita_movimientos" }, () =>
        clienteConsultas.invalidateQueries({ queryKey: CLAVE_MIS_VISITAS }),
      )
      .subscribe();
    return () => {
      void supabase.removeChannel(canal);
    };
  }, [clienteConsultas]);

  const responder = useMutation({
    mutationFn: ({ invitadoId, aprobar }: { invitadoId: string; aprobar: boolean }) =>
      responderSolicitud(invitadoId, aprobar, aprobar ? null : "Rechazada desde Mis visitas"),
    onSuccess: () => clienteConsultas.invalidateQueries({ queryKey: CLAVE_MIS_VISITAS }),
    onError: (e) => toast.error(mensajeError(e)),
  });

  const { solicitudes, hoy, proximas } = useMemo(() => {
    const visitas = agruparVisitas(data ?? []);
    const dia = hoyCostaRica();
    const solicitudes = visitas.filter((v) =>
      v.invitados.some((i) => i.invitado_estado === "SOLICITADO"),
    );
    const activas = visitas.filter((v) => estadoVisita(v, dia) !== "FINALIZADA");
    const hoy = activas.filter((v) => v.fecha_desde <= dia && dia <= v.fecha_hasta);
    const proximas = activas.filter((v) => v.fecha_desde > dia);
    return { solicitudes, hoy, proximas };
  }, [data]);

  if (isPending) return <Cargando texto="Cargando sus visitas…" />;
  if (isError)
    return (
      <div className="aviso aviso-error">{mensajeError(error)}</div>
    );

  const sinVisitas = solicitudes.length === 0 && hoy.length === 0 && proximas.length === 0;

  return (
    <div className="flex flex-col gap-6">
      <div className="flex items-center justify-between gap-4">
        <h1 className="text-xl font-semibold">Mis visitas</h1>
        <button type="button" className="boton boton-primario" onClick={() => navegar("/agendar")}>
          <CalendarPlus aria-hidden="true" size={18} />
          Agendar visita
        </button>
      </div>

      {solicitudes.length > 0 && (
        <section className="flex flex-col gap-3">
          <h2 className="text-sm font-semibold text-muted">Solicitudes de ingreso sin cita</h2>
          {solicitudes.map((visita) =>
            visita.invitados
              .filter((i) => i.invitado_estado === "SOLICITADO")
              .map((invitado) => (
                <div key={invitado.invitado_id} className="tarjeta flex items-center gap-3 p-4">
                  <div className="min-w-0 flex-1">
                    <p className="font-semibold">{invitado.visitante_nombre}</p>
                    <p className="text-sm text-muted">
                      Quiere ingresar ahora a {visita.sitio_nombre}
                    </p>
                  </div>
                  <button
                    type="button"
                    className="boton boton-icono"
                    aria-label="Rechazar"
                    disabled={responder.isPending}
                    onClick={() => responder.mutate({ invitadoId: invitado.invitado_id, aprobar: false })}
                  >
                    <X aria-hidden="true" />
                  </button>
                  <button
                    type="button"
                    className="boton boton-primario boton-icono"
                    aria-label="Aprobar"
                    disabled={responder.isPending}
                    onClick={() => responder.mutate({ invitadoId: invitado.invitado_id, aprobar: true })}
                  >
                    <Check aria-hidden="true" />
                  </button>
                </div>
              )),
          )}
        </section>
      )}

      {sinVisitas ? (
        <div className="estado-vacio">
          <span className="estado-vacio-icono">
            <CalendarX2 aria-hidden="true" size={28} />
          </span>
          <h2 className="text-lg font-semibold">Todavía no tiene visitas agendadas</h2>
          <p>Use el botón &ldquo;Agendar visita&rdquo; de arriba para que la persona pueda ingresar el día que la espera.</p>
        </div>
      ) : (
        <>
          <BloqueVisitas titulo="Hoy" visitas={hoy} onAbrir={(id) => navegar(`/visitas/${id}`)} />
          <BloqueVisitas titulo="Próximas" visitas={proximas} onAbrir={(id) => navegar(`/visitas/${id}`)} />
        </>
      )}
    </div>
  );
}

function BloqueVisitas({
  titulo,
  visitas,
  onAbrir,
}: {
  titulo: string;
  visitas: VisitaAgrupada[];
  onAbrir: (id: string) => void;
}) {
  if (visitas.length === 0) return null;
  return (
    <section className="flex flex-col gap-3">
      <h2 className="text-sm font-semibold text-muted">{titulo}</h2>
      <div className="flex flex-col gap-3">
        {visitas.map((visita) => (
          <TarjetaVisita key={visita.visita_id} visita={visita} onAbrir={() => onAbrir(visita.visita_id)} />
        ))}
      </div>
    </section>
  );
}

function TarjetaVisita({ visita, onAbrir }: { visita: VisitaAgrupada; onAbrir: () => void }) {
  return (
    <button type="button" onClick={onAbrir} className="tarjeta flex flex-col gap-3 p-4 text-left">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <p className="font-semibold">
            {fechaLegible(visita.fecha_desde)}
            {visita.fecha_hasta !== visita.fecha_desde ? ` – ${fechaLegible(visita.fecha_hasta)}` : ""}
          </p>
          <p className="mt-1 flex items-center gap-1.5 text-sm text-muted">
            <MapPin aria-hidden="true" size={14} />
            {visita.sitio_nombre} · {horaLegible(visita.hora_desde)}–{horaLegible(visita.hora_hasta)}
          </p>
        </div>
        <ChevronRight aria-hidden="true" className="mt-1 shrink-0 text-muted" size={18} />
      </div>
      <div className="flex flex-wrap items-center gap-2 text-sm">
        <Users aria-hidden="true" size={14} className="text-muted" />
        {visita.invitados.map((invitado) => (
          <span key={invitado.invitado_id} className={`chip ${claseEstadoInvitado(invitado.invitado_estado)}`}>
            {invitado.visitante_nombre.split(" ")[0]} · {etiquetaEstadoInvitado(invitado)}
          </span>
        ))}
      </div>
    </button>
  );
}

function claseEstadoInvitado(estado: string): string {
  if (estado === "EN_SITIO") return "chip-exito";
  if (estado === "FINALIZADO" || estado === "FUERA") return "chip-neutro";
  if (estado === "CANCELADA" || estado === "RECHAZADO" || estado === "NO_SE_PRESENTO") return "chip-error";
  if (estado === "SOLICITADO") return "chip-advertencia";
  return "chip-neutro";
}

function etiquetaEstadoInvitado(invitado: { invitado_estado: string; ultima_entrada: string | null; ultimo_gafete_numero: number | null }): string {
  const { invitado_estado, ultima_entrada, ultimo_gafete_numero } = invitado;
  if (invitado_estado === "EN_SITIO" && ultima_entrada) {
    const gafete = ultimo_gafete_numero != null ? ` · gafete ${ultimo_gafete_numero}` : "";
    return `Llegó ${horaCostaRicaDe(ultima_entrada)}${gafete}`;
  }
  if (invitado_estado === "FUERA" && ultima_entrada) return `Salió ${horaCostaRicaDe(ultima_entrada)}`;
  if (invitado_estado === "PROGRAMADO") return "Esperando";
  if (invitado_estado === "FINALIZADO") return "Finalizó";
  if (invitado_estado === "NO_SE_PRESENTO") return "No se presentó";
  if (invitado_estado === "CANCELADA") return "Cancelada";
  if (invitado_estado === "SOLICITADO") return "Sin cita";
  if (invitado_estado === "APROBADO") return "Aprobado";
  if (invitado_estado === "RECHAZADO") return "Rechazado";
  return invitado_estado;
}
