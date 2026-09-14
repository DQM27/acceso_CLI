import { useState } from "react";
import { hoyCostaRica } from "../fecha";
import SelectorFechas from "./SelectorFechas";

function partirFecha(ymd: string): { dia: string; mes: string; anio: string } {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(ymd);
  if (!m) return { dia: "", mes: "", anio: "" };
  const [, anio, mes, dia] = m;
  return { dia: dia ?? "", mes: mes ?? "", anio: anio ?? "" };
}

/** `null` si los 3 segmentos todavía no forman una fecha real (incompleta,
 * fuera de rango, o un día que no existe en ese mes -- ej. 31 de febrero). */
function unirFecha(dia: string, mes: string, anio: string): string | null {
  if (!/^\d{1,2}$/.test(dia) || !/^\d{1,2}$/.test(mes) || !/^\d{4}$/.test(anio))
    return null;
  const d = Number(dia);
  const mo = Number(mes);
  const y = Number(anio);
  if (mo < 1 || mo > 12 || d < 1 || d > 31) return null;
  const fecha = new Date(y, mo - 1, d);
  if (fecha.getFullYear() !== y || fecha.getMonth() !== mo - 1 || fecha.getDate() !== d)
    return null;
  return `${String(y).padStart(4, "0")}-${String(mo).padStart(2, "0")}-${String(d).padStart(2, "0")}`;
}

/**
 * Día/Mes/Año como 3 campos de texto separados, en vez de un único
 * `<input type="date">` -- el orden en que un navegador muestra ese input
 * nativo depende del idioma/región configurado en el NAVEGADOR de cada
 * visitante (no del `lang` de esta página), así que no hay forma de
 * garantizar día/mes/año para todos con un solo input nativo. Con 3 campos
 * fijos el orden queda fijo siempre, sin depender de eso -- mismo patrón
 * que usan GOV.UK Design System y USWDS para este problema exacto.
 *
 * Guarda cada segmento en estado local (no en el valor `YYYY-MM-DD` del
 * padre) para permitir estados intermedios mientras se tipea (ej. "3" en
 * día, todavía sin mes/año) sin intentar propagar una fecha inválida;
 * recién avisa al padre (`onCambiar`) cuando los 3 segmentos juntos forman
 * una fecha real. No hay avance automático de foco entre los 3 campos a
 * propósito -- es una práctica de accesibilidad desaconsejada (GOV.UK no
 * lo hace tampoco): sorprende a quien usa lector de pantalla y estorba a
 * quien se equivoca y quiere corregir.
 */
function CampoFechaSegmentada({
  etiqueta,
  valor,
  onCambiar,
  error,
}: {
  etiqueta: string;
  valor: string;
  onCambiar: (ymd: string) => void;
  error?: string;
}) {
  const [segmentos, setSegmentos] = useState(() => partirFecha(valor));
  // Ajusta el estado durante el render en vez de en un efecto (patrón
  // recomendado de React para "derivar estado de un prop que cambió" --
  // un efecto haría un ciclo de render extra de más).
  const [valorPrevio, setValorPrevio] = useState(valor);
  if (valor !== valorPrevio) {
    setValorPrevio(valor);
    setSegmentos(partirFecha(valor));
  }

  function actualizar(campo: "dia" | "mes" | "anio", texto: string) {
    const limpio = texto.replace(/\D/g, "").slice(0, campo === "anio" ? 4 : 2);
    const siguiente = { ...segmentos, [campo]: limpio };
    setSegmentos(siguiente);
    const combinado = unirFecha(siguiente.dia, siguiente.mes, siguiente.anio);
    if (combinado) onCambiar(combinado);
  }

  const idError = error ? `error-fecha-${etiqueta.toLowerCase()}` : undefined;

  return (
    <fieldset className="campo-fecha-segmentada">
      <legend>{etiqueta}</legend>
      <div className="campo-fecha-segmentos">
        <label>
          <span>Día</span>
          <input
            type="text"
            inputMode="numeric"
            pattern="[0-9]*"
            maxLength={2}
            autoComplete="off"
            className="form-control"
            placeholder="DD"
            value={segmentos.dia}
            aria-invalid={!!error}
            aria-describedby={idError}
            onChange={(e) => actualizar("dia", e.target.value)}
          />
        </label>
        <label>
          <span>Mes</span>
          <input
            type="text"
            inputMode="numeric"
            pattern="[0-9]*"
            maxLength={2}
            autoComplete="off"
            className="form-control"
            placeholder="MM"
            value={segmentos.mes}
            aria-invalid={!!error}
            aria-describedby={idError}
            onChange={(e) => actualizar("mes", e.target.value)}
          />
        </label>
        <label>
          <span>Año</span>
          <input
            type="text"
            inputMode="numeric"
            pattern="[0-9]*"
            maxLength={4}
            autoComplete="off"
            className="form-control"
            placeholder="AAAA"
            value={segmentos.anio}
            aria-invalid={!!error}
            aria-describedby={idError}
            onChange={(e) => actualizar("anio", e.target.value)}
          />
        </label>
      </div>
      {error && (
        <span className="error-campo" id={idError}>
          {error}
        </span>
      )}
    </fieldset>
  );
}

/**
 * Fechas de la visita: Desde/Hasta como 3 campos (Día/Mes/Año) cada una --
 * fuente de verdad primaria, siempre visibles, 100% operables por
 * teclado -- más el calendario visual (`SelectorFechas`) como
 * complemento, sincronizados en ambas direcciones por el mismo
 * `onCambiar`. Antes el único camino era arrastrar sobre un calendario de
 * FullCalendar, lo que bloqueaba por completo a quien navega por teclado
 * (hallazgo de la auditoría de accesibilidad); el input nativo
 * `<input type="date">` que reemplazó eso primero tenía a su vez un
 * problema distinto -- su orden día/mes/año depende del navegador de cada
 * visitante, no de esta app -- por eso el paso final a 3 campos fijos.
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
  return (
    <div>
      <div className="dos-columnas">
        <CampoFechaSegmentada
          etiqueta="Desde"
          valor={desde}
          error={erroresDesde}
          onCambiar={(ymd) => onCambiar(ymd, hasta)}
        />
        <CampoFechaSegmentada
          etiqueta="Hasta"
          valor={hasta}
          error={erroresHasta}
          onCambiar={(ymd) => onCambiar(desde, ymd)}
        />
      </div>
      <p className="ayuda-campo" style={{ marginTop: "8px" }}>
        También podés elegir en el calendario: un click selecciona un día, un
        segundo click en un día posterior arma el rango.
      </p>
      <SelectorFechas
        desde={desde || hoyCostaRica()}
        hasta={hasta || hoyCostaRica()}
        onCambiar={onCambiar}
      />
    </div>
  );
}
