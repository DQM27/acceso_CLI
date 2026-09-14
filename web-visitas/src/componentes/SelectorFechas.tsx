import { useState } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { hoyCostaRica } from "../fecha";

const DIAS_SEMANA = ["lun", "mar", "mié", "jue", "vie", "sáb", "dom"];

function aYMD(fecha: Date): string {
  const y = fecha.getFullYear();
  const m = String(fecha.getMonth() + 1).padStart(2, "0");
  const d = String(fecha.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

/** 6 semanas (42 días) empezando el lunes on/antes del día 1 -- mismo
 * tamaño de grilla que ya se veía con FullCalendar, para no achicar la
 * altura visual del calendario. */
function construirGrilla(año: number, mes: number): Date[] {
  const primero = new Date(año, mes, 1);
  const corrimientoLunes = (primero.getDay() + 6) % 7;
  const inicio = new Date(año, mes, 1 - corrimientoLunes);
  return Array.from(
    { length: 42 },
    (_, i) => new Date(inicio.getFullYear(), inicio.getMonth(), inicio.getDate() + i),
  );
}

/**
 * Selector de `fecha_desde`/`fecha_hasta` -- calendario propio, sin
 * librería externa. Reemplaza a la versión anterior basada en FullCalendar
 * (@fullcalendar/react): combinada con `<Activity>` en el wizard de
 * "Nueva cita" (`NuevaCita.tsx`), el wrapper de FullCalendar reinicializaba
 * su vista cada vez que Activity volvía a correr sus efectos al mostrar el
 * paso de nuevo -- perdía el mes al que el usuario había navegado. Un
 * calendario hecho a mano guarda el mes mostrado en `useState` normal, que
 * sí sobrevive ese ciclo sin problema. También evita por completo el
 * parche de CSP que necesitaba FullCalendar (fuente de íconos como
 * `data:` URI) y el bundle de esa librería.
 *
 * Interacción: un click elige un solo día; un segundo click en un día
 * posterior extiende el rango hasta ahí (equivalente al "arrastrar" de
 * antes, pero además operable con teclado -- cada día es un `<button>`
 * real). `CampoFechas.tsx` sigue siendo la vía principal (los dos
 * `<input type="date">`); este calendario es el complemento visual.
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
  const hoy = hoyCostaRica();
  const inicial = new Date(`${desde || hoy}T00:00:00`);
  const [año, setAño] = useState(inicial.getFullYear());
  const [mes, setMes] = useState(inicial.getMonth());

  function cambiarMes(delta: number) {
    const referencia = new Date(año, mes + delta, 1);
    setAño(referencia.getFullYear());
    setMes(referencia.getMonth());
  }

  function alElegir(fechaYMD: string) {
    if (fechaYMD < hoy) return;
    if (desde && hasta && desde === hasta && fechaYMD > desde) {
      onCambiar(desde, fechaYMD);
    } else {
      onCambiar(fechaYMD, fechaYMD);
    }
  }

  const dias = construirGrilla(año, mes);
  const tituloCrudo = new Intl.DateTimeFormat("es-CR", {
    month: "long",
    year: "numeric",
  }).format(new Date(año, mes, 1));
  // Sólo la primera letra -- `text-transform: capitalize` en CSS pondría
  // mayúscula también en "de" ("Septiembre De 2026"), incorrecto en español.
  const titulo = tituloCrudo.charAt(0).toUpperCase() + tituloCrudo.slice(1);

  return (
    <div className="selector-fechas">
      <div className="selector-fechas-toolbar">
        <div className="selector-fechas-nav">
          <button
            type="button"
            className="btn btn-sm btn-outline-secondary"
            aria-label="Mes anterior"
            onClick={() => cambiarMes(-1)}
          >
            <ChevronLeft aria-hidden="true" />
          </button>
          <button
            type="button"
            className="btn btn-sm btn-outline-secondary"
            aria-label="Mes siguiente"
            onClick={() => cambiarMes(1)}
          >
            <ChevronRight aria-hidden="true" />
          </button>
        </div>
        <span className="selector-fechas-titulo">{titulo}</span>
        <button
          type="button"
          className="btn btn-sm btn-outline-secondary"
          onClick={() => {
            const hoyDate = new Date(`${hoy}T00:00:00`);
            setAño(hoyDate.getFullYear());
            setMes(hoyDate.getMonth());
          }}
        >
          Hoy
        </button>
      </div>
      <div className="selector-fechas-grilla" role="grid" aria-label={titulo}>
        <div className="selector-fechas-fila selector-fechas-encabezado" role="row">
          {DIAS_SEMANA.map((d) => (
            <span key={d} role="columnheader">
              {d}
            </span>
          ))}
        </div>
        {Array.from({ length: 6 }, (_, fila) => (
          <div className="selector-fechas-fila" role="row" key={fila}>
            {dias.slice(fila * 7, fila * 7 + 7).map((dia) => {
              const ymd = aYMD(dia);
              const enMes = dia.getMonth() === mes;
              const enRango = ymd >= desde && ymd <= hasta;
              const esLimite = ymd === desde || ymd === hasta;
              const pasado = ymd < hoy;
              return (
                <button
                  type="button"
                  key={ymd}
                  role="gridcell"
                  disabled={pasado}
                  aria-current={ymd === hoy ? "date" : undefined}
                  aria-pressed={enRango}
                  className={[
                    "selector-fechas-dia",
                    !enMes && "selector-fechas-dia-otro-mes",
                    enRango && "selector-fechas-dia-rango",
                    esLimite && "selector-fechas-dia-limite",
                    ymd === hoy && "selector-fechas-dia-hoy",
                  ]
                    .filter(Boolean)
                    .join(" ")}
                  onClick={() => alElegir(ymd)}
                >
                  {dia.getDate()}
                </button>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}
