import { Check } from "lucide-react";

/**
 * Indicador de progreso del wizard de "Nueva cita" -- reemplaza el viejo
 * `.indicador-paso` que además se ocultaba por completo en móvil
 * (`@media (max-width:540px)`), justo donde más se usa esta pantalla
 * (hallazgo de alto impacto de la auditoría). Visible en todos los
 * breakpoints: en pantallas angostas los números quedan más juntos y las
 * etiquetas se acortan por CSS, pero nunca desaparecen.
 */
export default function PasoWizard({
  pasos,
  actual,
}: {
  pasos: string[];
  actual: number;
}) {
  return (
    <nav aria-label="Progreso de la cita" className="paso-wizard">
      <span className="sr-only" aria-live="polite">
        Paso {actual + 1} de {pasos.length}: {pasos[actual]}
      </span>
      <ol aria-hidden="true">
        {pasos.map((etiqueta, i) => {
          const estado =
            i < actual ? "completado" : i === actual ? "activo" : "pendiente";
          return (
            <li key={etiqueta} className={`paso-wizard-item paso-wizard-${estado}`}>
              <span
                className="paso-wizard-numero"
                aria-current={estado === "activo" ? "step" : undefined}
              >
                {estado === "completado" ? (
                  <Check aria-hidden="true" />
                ) : (
                  i + 1
                )}
              </span>
              <span className="paso-wizard-etiqueta">{etiqueta}</span>
              {i < pasos.length - 1 && (
                <span className="paso-wizard-linea" />
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
