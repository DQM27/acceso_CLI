import { useEffect, useRef } from "react";
import "../lib/parcheCspFullcalendar";
import FullCalendar from "@fullcalendar/react";
import dayGridPlugin from "@fullcalendar/daygrid";
import interactionPlugin from "@fullcalendar/interaction";
import type { CalendarApi, DateSelectArg } from "@fullcalendar/core";
import esLocale from "@fullcalendar/core/locales/es";
import { hoyCostaRica } from "../fecha";

/** `FullCalendar` es `end` exclusivo tanto al seleccionar como al marcar la
 * selección por API -- una cita del 10 al 12 necesita `end: "13"` para que
 * el día 12 quede pintado como parte del rango. */
function finExclusivo(fechaYMD: string): string {
  const fecha = new Date(`${fechaYMD}T00:00:00`);
  fecha.setDate(fecha.getDate() + 1);
  return fecha.toISOString().slice(0, 10);
}

/**
 * Selector de `fecha_desde`/`fecha_hasta` por click-y-arrastre sobre un
 * calendario, en vez de dos campos de texto sueltos -- pedido del usuario
 * ("marcar fecha con doble click y hacer el rellenado", inspirado en
 * apps de calendario tipo Supershift). Un solo click selecciona un único
 * día; arrastrar selecciona un rango.
 *
 * Ver `index.html` (el `<style data-fullcalendar>` vacío) y
 * `public/_headers` (el hash de ese elemento en la CSP) -- FullCalendar
 * inyecta su CSS por JS, lo que la CSP estricta de esta app bloquearía sin
 * ese enganche.
 */
export default function SelectorFechas({
  desde,
  hasta,
  onCambiar,
}: {
  desde: string;
  hasta: string;
  onCambiar: (desde: string, hasta: string) => void;
}) {
  const apiRef = useRef<CalendarApi | null>(null);

  // Refleja en el calendario un cambio de fechas que no vino de acá mismo
  // (valor inicial, o si el formulario las corrige por otra razón) -- sin
  // esto, seleccionar en el calendario movía el estado del formulario pero
  // el resaltado visual se quedaba en la selección anterior.
  useEffect(() => {
    apiRef.current?.select(desde, finExclusivo(hasta));
  }, [desde, hasta]);

  function alSeleccionar(info: DateSelectArg) {
    const nuevaHasta = new Date(info.end);
    nuevaHasta.setDate(nuevaHasta.getDate() - 1);
    onCambiar(info.startStr, nuevaHasta.toISOString().slice(0, 10));
  }

  return (
    <div className="selector-fechas">
      <FullCalendar
        ref={(instancia) => {
          apiRef.current = instancia?.getApi() ?? null;
        }}
        plugins={[dayGridPlugin, interactionPlugin]}
        initialView="dayGridMonth"
        headerToolbar={{ left: "prev,next today", center: "title", right: "" }}
        buttonText={{ today: "Hoy" }}
        locale={esLocale}
        height="auto"
        selectable
        selectMirror
        unselectAuto={false}
        validRange={{ start: hoyCostaRica() }}
        select={alSeleccionar}
        initialDate={desde}
      />
    </div>
  );
}
