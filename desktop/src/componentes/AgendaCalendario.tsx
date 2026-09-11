import { useMemo, useState } from "react";
import FullCalendar from "@fullcalendar/react";
import dayGridPlugin from "@fullcalendar/daygrid";
import listPlugin from "@fullcalendar/list";
import esLocale from "@fullcalendar/core/locales/es";
import type { EventClickArg, EventContentArg } from "@fullcalendar/core";
import Modal from "./Modal";
import type { AgendaVisitaResumen } from "../api";
import { textoFechaDDMMYYYY } from "../tiempo";

/** `FullCalendar` es `all-day` con `end` exclusivo -- una cita del 10 al 12
 * necesita `end: "13"` para que el día 12 completo quede pintado. Sumar un
 * día a `fecha_hasta` acá, no tocar el dato real que sigue siendo inclusive
 * en todos lados (SQLite, Supabase, la RPC). */
export function finExclusivo(fechaHastaYMD: string): string {
  const fecha = new Date(`${fechaHastaYMD}T00:00:00`);
  fecha.setDate(fecha.getDate() + 1);
  return fecha.toISOString().slice(0, 10);
}

export function tituloEvento(fila: AgendaVisitaResumen): string {
  return fila.empresa ? `${fila.nombre} · ${fila.empresa}` : fila.nombre;
}

/**
 * Calendario de citas programadas (docs/plan-control-visitas.md, sección
 * Agenda) -- de sólo lectura, no dispara ningún check-in. Cada fila de
 * `AgendaVisitaResumen` (una por visitante) es un evento de todo el día que
 * cubre `[fecha_desde, fecha_hasta]`. Click en un evento abre el detalle
 * completo en un modal -- FullCalendar no tiene espacio para mostrar
 * cédula/motivo/anfitrión en la celda del mes sin amontonar todo.
 */
export default function AgendaCalendario({ filas }: { filas: AgendaVisitaResumen[] }) {
  const [seleccionado, setSeleccionado] = useState<AgendaVisitaResumen | null>(null);

  const eventos = useMemo(
    () =>
      filas.map((fila) => ({
        id: String(fila.cita_id) + "-" + fila.cedula,
        title: tituloEvento(fila),
        start: fila.fecha_desde,
        end: finExclusivo(fila.fecha_hasta),
        allDay: true,
        // `estado` viaja en `extendedProps` -- FullCalendar no sabe nada de
        // nuestro dominio, sólo lo usamos nosotros mismos en
        // `eventContent`/`eventClick` de acá abajo.
        extendedProps: { fila },
        classNames: fila.estado === "Cancelada" ? ["agenda-evento-cancelada"] : ["agenda-evento-vigente"],
      })),
    [filas],
  );

  return (
    <div className="agenda-calendario" style={{ height: "100%", display: "flex", flexDirection: "column" }}>
      <FullCalendar
        plugins={[dayGridPlugin, listPlugin]}
        initialView="dayGridMonth"
        headerToolbar={{
          left: "prev,next today",
          center: "title",
          right: "dayGridMonth,listMonth",
        }}
        buttonText={{ today: "Hoy", month: "Mes", list: "Lista" }}
        locale={esLocale}
        height="100%"
        events={eventos}
        dayMaxEvents={3}
        eventContent={(argumento: EventContentArg) => (
          <span className="agenda-evento-etiqueta">{argumento.event.title}</span>
        )}
        eventClick={(argumento: EventClickArg) => {
          setSeleccionado(argumento.event.extendedProps.fila as AgendaVisitaResumen);
        }}
      />

      {seleccionado && (
        <Modal titulo={seleccionado.nombre} onCerrar={() => setSeleccionado(null)}>
          <div style={{ display: "flex", flexDirection: "column", gap: "0.6rem" }}>
            <DetalleFila etiqueta="Cédula" valor={seleccionado.cedula} />
            <DetalleFila etiqueta="Empresa" valor={seleccionado.empresa ?? "—"} />
            <DetalleFila etiqueta="Placa" valor={seleccionado.placa_vehiculo ?? "—"} />
            <DetalleFila etiqueta="Anfitrión" valor={seleccionado.anfitrion_nombre} />
            <DetalleFila etiqueta="Motivo" valor={seleccionado.motivo ?? "—"} />
            <DetalleFila
              etiqueta="Vigencia"
              valor={`${textoFechaDDMMYYYY(seleccionado.fecha_desde)} — ${textoFechaDDMMYYYY(seleccionado.fecha_hasta)}`}
            />
            {seleccionado.hora_estimada && (
              <DetalleFila etiqueta="Hora estimada" valor={seleccionado.hora_estimada.slice(0, 5)} />
            )}
            <DetalleFila
              etiqueta="Estado"
              valor={seleccionado.estado === "Cancelada" ? "Cancelada" : "Vigente"}
              color={seleccionado.estado === "Cancelada" ? "var(--error)" : "var(--exito)"}
            />
          </div>
        </Modal>
      )}
    </div>
  );
}

function DetalleFila({ etiqueta, valor, color }: { etiqueta: string; valor: string; color?: string }) {
  return (
    <div style={{ display: "flex", justifyContent: "space-between", gap: "1rem" }}>
      <span style={{ color: "var(--muted)" }}>{etiqueta}</span>
      <span style={{ color: color ?? "var(--texto)", fontWeight: 600, textAlign: "right" }}>{valor}</span>
    </div>
  );
}
