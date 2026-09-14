import { Trash2, Users } from "lucide-react";
import type { ReactNode } from "react";
import type { FormularioCita } from "../dominio";

type Visitante = FormularioCita["visitantes"][number];

/** A partir de cuántos visitantes se empieza a colapsar por defecto -- con
 * menos, ver todo de un vistazo es más rápido que abrir uno por uno. */
const UMBRAL_COLAPSO = 3;

/**
 * Una tarjeta de visitante dentro del paso "Visitantes" de `NuevaCita.tsx`
 * -- extraída a su propio componente para poder colapsarla con
 * `<details>/<summary>` nativo (sin JS de Bootstrap ni su `.accordion`,
 * que además pinta el ícono de flecha con un `background-image: data:`
 * que la CSP estricta de esta app bloquea). Con grupos grandes (más de
 * `UMBRAL_COLAPSO`), las tarjetas ya completas y sin error quedan
 * colapsadas a una sola línea de resumen -- deja expandida la que se
 * recién agregó (probablemente la que se está llenando) y cualquiera con
 * error, para que no haya que ir abriendo una por una a buscar el
 * problema.
 */
export default function VisitanteFormulario({
  indice,
  total,
  persona,
  esUltimo,
  tieneError,
  onCambiar,
  onQuitar,
  mensajeCampo,
  atributos,
}: {
  indice: number;
  total: number;
  persona: Visitante;
  esUltimo: boolean;
  tieneError: boolean;
  onCambiar: (campo: keyof Visitante, valor: string) => void;
  onQuitar: (() => void) | null;
  mensajeCampo: (clave: string) => ReactNode;
  atributos: (clave: string) => {
    "aria-invalid": boolean;
    "aria-describedby": string | undefined;
  };
}) {
  // Sólo se colapsa una tarjeta ya COMPLETA (los dos campos obligatorios
  // cargados) -- si colapsara apenas falta cualquiera de los dos, escribir
  // la primera letra del nombre podría plegar la tarjeta a mitad de tecleo
  // (el nombre ya no estaría vacío, pero la cédula todavía sí).
  const completo =
    persona.nombre.trim().length > 0 && persona.cedula.trim().length > 0;
  const abierto = total <= UMBRAL_COLAPSO || esUltimo || tieneError || !completo;
  const resumen = [persona.nombre, persona.cedula].filter(Boolean).join(" · ");

  return (
    <details className="visitante-formulario" open={abierto}>
      <summary className="visitante-resumen">
        <Users aria-hidden="true" />
        <span>
          Visitante {indice + 1}
          {resumen && <span className="texto-secundario"> · {resumen}</span>}
        </span>
        {tieneError && (
          <span className="badge text-bg-danger">Revisar</span>
        )}
      </summary>
      <div className="visitante-cuerpo">
        {onQuitar && (
          <div className="visitante-acciones">
            <button
              type="button"
              className="btn btn-sm btn-link text-danger"
              aria-label={`Quitar visitante ${indice + 1}`}
              onClick={onQuitar}
            >
              <Trash2 aria-hidden="true" />
              Quitar
            </button>
          </div>
        )}
        <div className="dos-columnas">
          <label className="campo">
            Nombre completo <span className="obligatorio">*</span>
            <input
              className="form-control"
              autoComplete="off"
              value={persona.nombre}
              maxLength={150}
              required
              {...atributos(`visitantes.${indice}.nombre`)}
              onChange={(e) => onCambiar("nombre", e.target.value)}
            />
            {mensajeCampo(`visitantes.${indice}.nombre`)}
          </label>
          <label className="campo">
            Cédula o documento <span className="obligatorio">*</span>
            <input
              className="form-control"
              autoComplete="off"
              spellCheck={false}
              placeholder="Por ejemplo: 1-2345-6789"
              value={persona.cedula}
              maxLength={60}
              required
              {...atributos(`visitantes.${indice}.cedula`)}
              onChange={(e) => onCambiar("cedula", e.target.value)}
            />
            {mensajeCampo(`visitantes.${indice}.cedula`)}
          </label>
          <label className="campo">
            Empresa <span className="opcional">Opcional</span>
            <input
              className="form-control"
              autoComplete="off"
              value={persona.empresa}
              maxLength={150}
              {...atributos(`visitantes.${indice}.empresa`)}
              onChange={(e) => onCambiar("empresa", e.target.value)}
            />
            {mensajeCampo(`visitantes.${indice}.empresa`)}
          </label>
          <label className="campo">
            Placa del vehículo <span className="opcional">Opcional</span>
            <input
              className="form-control"
              autoComplete="off"
              value={persona.placa_vehiculo}
              maxLength={20}
              {...atributos(`visitantes.${indice}.placa_vehiculo`)}
              onChange={(e) => onCambiar("placa_vehiculo", e.target.value)}
            />
            {mensajeCampo(`visitantes.${indice}.placa_vehiculo`)}
          </label>
        </div>
      </div>
    </details>
  );
}
