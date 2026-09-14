import { hoyCostaRica } from "../fecha";
import SelectorFechas from "./SelectorFechas";

/**
 * Fechas de la visita: dos `<input type="date">` nativos como fuente de
 * verdad primaria -- siempre visibles, 100% operables por teclado, con el
 * selector nativo del sistema en el celular -- más el calendario visual
 * (`SelectorFechas`) como complemento, sincronizados en ambas direcciones
 * por el mismo `onCambiar`. Antes el único camino era arrastrar sobre un
 * calendario de FullCalendar, lo que bloqueaba por completo a quien navega
 * por teclado (hallazgo de la auditoría de accesibilidad).
 */
export default function CampoFechas({
  desde,
  hasta,
  onCambiar,
  erroresDesde,
  erroresHasta,
}: {
  desde: string;
  hasta: string;
  onCambiar: (desde: string, hasta: string) => void;
  erroresDesde?: string;
  erroresHasta?: string;
}) {
  const hoy = hoyCostaRica();
  return (
    <div>
      <div className="dos-columnas">
        <label className="campo">
          Desde
          <input
            type="date"
            className="form-control"
            value={desde}
            min={hoy}
            aria-invalid={!!erroresDesde}
            aria-describedby={erroresDesde ? "error-fecha_desde" : undefined}
            onChange={(e) => onCambiar(e.target.value, hasta)}
          />
          {erroresDesde && (
            <span className="error-campo" id="error-fecha_desde">
              {erroresDesde}
            </span>
          )}
        </label>
        <label className="campo">
          Hasta
          <input
            type="date"
            className="form-control"
            value={hasta}
            min={desde || hoy}
            aria-invalid={!!erroresHasta}
            aria-describedby={erroresHasta ? "error-fecha_hasta" : undefined}
            onChange={(e) => onCambiar(desde, e.target.value)}
          />
          {erroresHasta && (
            <span className="error-campo" id="error-fecha_hasta">
              {erroresHasta}
            </span>
          )}
        </label>
      </div>
      <p className="ayuda-campo" style={{ marginTop: "8px" }}>
        También podés elegir en el calendario: un click selecciona un día, un
        segundo click en un día posterior arma el rango.
      </p>
      <SelectorFechas desde={desde} hasta={hasta} onCambiar={onCambiar} />
    </div>
  );
}
