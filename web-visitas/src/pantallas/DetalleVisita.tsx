import { useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { ArrowLeft, Calendar, Copy, MapPin, Pencil, X } from "lucide-react";
import { cancelarVisita, duplicarVisita, editarVisita, listarMisVisitas, mensajeError } from "../api";
import { agruparVisitas } from "../dominio";
import { fechaLegible, horaCostaRicaDe, horaLegible } from "../fecha";
import { Cargando, Modal } from "../componentes/Comunes";

const CLAVE_MIS_VISITAS = ["mis-visitas"];

export default function DetalleVisita() {
  const { id: idParametro } = useParams<{ id: string }>();
  const id = idParametro ?? "";
  const navegar = useNavigate();
  const clienteConsultas = useQueryClient();
  const [confirmandoCancelar, setConfirmandoCancelar] = useState(false);
  const [editando, setEditando] = useState(false);

  const { data, isPending } = useQuery({
    queryKey: CLAVE_MIS_VISITAS,
    queryFn: ({ signal }) => listarMisVisitas(signal),
    staleTime: 15_000,
  });

  const visita = agruparVisitas(data ?? []).find((v) => v.visita_id === id);

  const cancelar = useMutation({
    mutationFn: () => cancelarVisita(id),
    onSuccess: () => {
      toast.success("Visita cancelada.");
      clienteConsultas.invalidateQueries({ queryKey: CLAVE_MIS_VISITAS });
      navegar("/");
    },
    onError: (e) => toast.error(mensajeError(e)),
  });

  const duplicar = useMutation({
    mutationFn: () => {
      const manana = new Date();
      manana.setDate(manana.getDate() + 1);
      const fecha = manana.toISOString().slice(0, 10);
      return duplicarVisita(id, crypto.randomUUID(), fecha, fecha);
    },
    onSuccess: () => {
      toast.success("Visita duplicada para mañana. Puede ajustar la fecha desde Mis visitas.");
      clienteConsultas.invalidateQueries({ queryKey: CLAVE_MIS_VISITAS });
      navegar("/");
    },
    onError: (e) => toast.error(mensajeError(e)),
  });

  if (isPending) return <Cargando texto="Cargando visita…" />;
  if (!visita)
    return (
      <div className="flex flex-col gap-4">
        <BotonVolver onClick={() => navegar("/")} />
        <p className="aviso aviso-info">No encontramos esta visita, o ya no tiene acceso a ella.</p>
      </div>
    );

  const nadieIngreso = !visita.invitados.some((i) =>
    ["EN_SITIO", "FUERA", "FINALIZADO"].includes(i.invitado_estado),
  );
  const puedeEditar = visita.visita_estado === "VIGENTE" && nadieIngreso;
  const puedeCancelar = visita.visita_estado === "VIGENTE";

  return (
    <div className="flex flex-col gap-6">
      <BotonVolver onClick={() => navegar("/")} />

      <div>
        <h1 className="text-xl font-semibold">
          {fechaLegible(visita.fecha_desde)}
          {visita.fecha_hasta !== visita.fecha_desde ? ` – ${fechaLegible(visita.fecha_hasta)}` : ""}
        </h1>
        <p className="mt-1 flex items-center gap-1.5 text-sm text-muted">
          <MapPin aria-hidden="true" size={14} />
          {visita.sitio_nombre} · {horaLegible(visita.hora_desde)}–{horaLegible(visita.hora_hasta)}
        </p>
        {visita.motivo && <p className="mt-2 text-sm">{visita.motivo}</p>}
      </div>

      <section className="tarjeta flex flex-col divide-y divide-borde">
        {visita.invitados.map((invitado) => (
          <div key={invitado.invitado_id} className="flex flex-col gap-1 p-4">
            <p className="font-medium">{invitado.visitante_nombre}</p>
            {invitado.visitante_empresa && <p className="text-sm text-muted">{invitado.visitante_empresa}</p>}
            <p className="mt-1 text-sm text-muted">
              {invitado.ultima_entrada
                ? `Entró ${horaCostaRicaDe(invitado.ultima_entrada)}${
                    invitado.ultima_salida ? ` · Salió ${horaCostaRicaDe(invitado.ultima_salida)}` : ""
                  }${invitado.ultimo_gafete_numero != null ? ` · Gafete ${invitado.ultimo_gafete_numero}` : ""}`
                : etiquetaEstado(invitado.invitado_estado)}
            </p>
          </div>
        ))}
      </section>

      {(puedeEditar || puedeCancelar) && (
        <div className="flex flex-wrap gap-2">
          {puedeEditar && (
            <button type="button" className="boton" onClick={() => setEditando(true)}>
              <Pencil aria-hidden="true" size={16} />
              Editar
            </button>
          )}
          <button type="button" className="boton" disabled={duplicar.isPending} onClick={() => duplicar.mutate()}>
            <Copy aria-hidden="true" size={16} />
            Duplicar
          </button>
          {puedeCancelar && (
            <button type="button" className="boton boton-peligro" onClick={() => setConfirmandoCancelar(true)}>
              <X aria-hidden="true" size={16} />
              Cancelar visita
            </button>
          )}
        </div>
      )}

      {confirmandoCancelar && (
        <Modal titulo="Cancelar visita" onCerrar={() => setConfirmandoCancelar(false)} ocupado={cancelar.isPending}>
          <p>
            ¿Cancelar la visita del {fechaLegible(visita.fecha_desde)} en {visita.sitio_nombre}? Las personas que
            todavía no llegaron no van a poder ingresar.
          </p>
          <div className="flex justify-end gap-2">
            <button type="button" className="boton" disabled={cancelar.isPending} onClick={() => setConfirmandoCancelar(false)}>
              Volver
            </button>
            <button
              type="button"
              className="boton boton-peligro"
              disabled={cancelar.isPending}
              onClick={() => cancelar.mutate()}
            >
              {cancelar.isPending ? "Cancelando…" : "Sí, cancelar"}
            </button>
          </div>
        </Modal>
      )}

      {editando && (
        <ModalEditar
          visitaId={visita.visita_id}
          fechaDesde={visita.fecha_desde}
          fechaHasta={visita.fecha_hasta}
          horaDesde={visita.hora_desde.slice(0, 5)}
          horaHasta={visita.hora_hasta.slice(0, 5)}
          motivo={visita.motivo ?? ""}
          requiereEscolta={visita.requiere_escolta}
          invitados={visita.invitados}
          onCerrar={() => setEditando(false)}
        />
      )}
    </div>
  );
}

function BotonVolver({ onClick }: { onClick: () => void }) {
  return (
    <button type="button" className="boton boton-fantasma boton-icono self-start" onClick={onClick} aria-label="Volver a Mis visitas">
      <ArrowLeft aria-hidden="true" />
    </button>
  );
}

function etiquetaEstado(estado: string): string {
  if (estado === "PROGRAMADO") return "Todavía no llega";
  if (estado === "NO_SE_PRESENTO") return "No se presentó";
  if (estado === "CANCELADA") return "Cancelada";
  if (estado === "SOLICITADO") return "Solicitud sin cita, pendiente de respuesta";
  if (estado === "APROBADO") return "Aprobado, todavía no llega";
  if (estado === "RECHAZADO") return "Rechazada";
  return estado;
}

function ModalEditar({
  visitaId,
  fechaDesde: fechaDesdeInicial,
  fechaHasta: fechaHastaInicial,
  horaDesde: horaDesdeInicial,
  horaHasta: horaHastaInicial,
  motivo: motivoInicial,
  requiereEscolta: requiereEscoltaInicial,
  invitados,
  onCerrar,
}: {
  visitaId: string;
  fechaDesde: string;
  fechaHasta: string;
  horaDesde: string;
  horaHasta: string;
  motivo: string;
  requiereEscolta: boolean;
  invitados: {
    tipo_documento: "CEDULA" | "DIMEX" | "PASAPORTE" | "OTRO";
    numero_documento: string;
    visitante_nombre: string;
    visitante_empresa: string | null;
    placa_vehiculo: string | null;
  }[];
  onCerrar: () => void;
}) {
  const clienteConsultas = useQueryClient();
  const [fechaDesde, setFechaDesde] = useState(fechaDesdeInicial);
  const [fechaHasta, setFechaHasta] = useState(fechaHastaInicial);
  const [horaDesde, setHoraDesde] = useState(horaDesdeInicial);
  const [horaHasta, setHoraHasta] = useState(horaHastaInicial);
  const [motivo, setMotivo] = useState(motivoInicial);
  const [requiereEscolta, setRequiereEscolta] = useState(requiereEscoltaInicial);

  const editar = useMutation({
    mutationFn: () =>
      editarVisita(visitaId, {
        fecha_desde: fechaDesde,
        fecha_hasta: fechaHasta,
        hora_desde: horaDesde,
        hora_hasta: horaHasta,
        tipo_visita: "",
        motivo,
        requiere_escolta: requiereEscolta,
        sitios: [],
        invitados: invitados.map((i) => ({
          tipo_documento: i.tipo_documento,
          numero_documento: i.numero_documento,
          nombre: i.visitante_nombre,
          empresa: i.visitante_empresa ?? "",
          telefono: "",
          correo: "",
          placa_vehiculo: i.placa_vehiculo ?? "",
        })),
      }),
    onSuccess: () => {
      toast.success("Visita actualizada.");
      clienteConsultas.invalidateQueries({ queryKey: CLAVE_MIS_VISITAS });
      onCerrar();
    },
    onError: (e) => toast.error(mensajeError(e)),
  });

  return (
    <Modal titulo="Editar visita" onCerrar={onCerrar} ocupado={editar.isPending}>
      <div className="grid grid-cols-2 gap-3">
        <label className="campo">
          Desde
          <input type="date" value={fechaDesde} onChange={(e) => setFechaDesde(e.target.value)} />
        </label>
        <label className="campo">
          Hasta
          <input type="date" value={fechaHasta} min={fechaDesde} onChange={(e) => setFechaHasta(e.target.value)} />
        </label>
        <label className="campo">
          Hora de entrada
          <input type="time" value={horaDesde} onChange={(e) => setHoraDesde(e.target.value)} />
        </label>
        <label className="campo">
          Hora de salida
          <input type="time" value={horaHasta} onChange={(e) => setHoraHasta(e.target.value)} />
        </label>
      </div>
      <label className="flex items-center gap-2 text-sm font-medium">
        <input type="checkbox" checked={requiereEscolta} onChange={(e) => setRequiereEscolta(e.target.checked)} />
        Esta visita necesita escolta
      </label>
      <label className="campo">
        Motivo
        <textarea value={motivo} onChange={(e) => setMotivo(e.target.value)} rows={3} />
      </label>
      <p className="campo-ayuda">
        <Calendar aria-hidden="true" size={14} className="mr-1 inline" />
        Para agregar o quitar personas, cancele esta visita y agende una nueva.
      </p>
      <div className="flex justify-end gap-2">
        <button type="button" className="boton" disabled={editar.isPending} onClick={onCerrar}>
          Cancelar
        </button>
        <button type="button" className="boton boton-primario" disabled={editar.isPending} onClick={() => editar.mutate()}>
          {editar.isPending ? "Guardando…" : "Guardar cambios"}
        </button>
      </div>
    </Modal>
  );
}
