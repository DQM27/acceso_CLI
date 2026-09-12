import { useMemo } from "react";
import "../lib/parcheCspFullcalendar";
import FullCalendar from "@fullcalendar/react";
import dayGridPlugin from "@fullcalendar/daygrid";
import listPlugin from "@fullcalendar/list";
import type { EventClickArg } from "@fullcalendar/core";
import esLocale from "@fullcalendar/core/locales/es";
import type { Cita } from "../dominio";
import { estadoCita } from "../fecha";

/** `FullCalendar` es `end` exclusivo -- una cita del 10 al 12 necesita
 * `end: "13"` para que el día 12 quede pintado como parte del rango. */
function finExclusivo(fechaYMD: string): string {
  const fecha = new Date(`${fechaYMD}T00:00:00`);
  fecha.setDate(fecha.getDate() + 1);
  return fecha.toISOString().slice(0, 10);
}

function tituloCita(cita: Cita): string {
  if (cita.motivo) return cita.motivo;
  const [primero] = cita.cita_visitantes;
  const n = cita.cita_visitantes.length;
  return n === 1 && primero ? `Visita de ${primero.nombre}` : `Visita de ${n} personas`;
}

/**
 * Calendario de "Mis citas" -- alternativa a la lista para quien prefiere
 * ver su agenda como un calendario real en vez de una lista de tarjetas
 * (pedido del usuario: "un calendario como el de la app [desktop]").
 * Reusa el mismo patrón que `SelectorFechas.tsx` (mes + lista, mismo
 * parche de CSP). Click en un evento reusa el mismo modal de detalle que
 * ya tiene la vista de lista -- no duplica esa UI.
 */
export default function CitasCalendario({
  citas,
  onSeleccionar,
}: {
  citas: Cita[];
  onSeleccionar: (cita: Cita) => void;
}) {
  const eventos = useMemo(
    () =>
      citas.map((cita) => {
        const estado = estadoCita(cita);
        return {
          id: cita.id,
          title: tituloCita(cita),
          start: cita.fecha_desde,
          end: finExclusivo(cita.fecha_hasta),
          allDay: true,
          extendedProps: { cita },
          classNames: [`citas-calendario-evento-${estado.toLowerCase()}`],
        };
      }),
    [citas],
  );

  return (
    <div className="citas-calendario">
      <FullCalendar
        plugins={[dayGridPlugin, listPlugin]}
        initialView="dayGridMonth"
        headerToolbar={{ left: "prev,next today", center: "title", right: "dayGridMonth,listMonth" }}
        buttonText={{ today: "Hoy", month: "Mes", list: "Lista" }}
        locale={esLocale}
        height="100%"
        events={eventos}
        dayMaxEvents={3}
        eventClick={(argumento: EventClickArg) => {
          onSeleccionar(argumento.event.extendedProps.cita as Cita);
        }}
      />
    </div>
  );
}
