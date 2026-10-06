import { useId, useState } from "react";
import * as Popover from "@radix-ui/react-popover";
import { DayPicker } from "react-day-picker";
import { es } from "react-day-picker/locale/es";
import { CalendarDays } from "lucide-react";
import { fechaCorta, rangoLegible } from "../fecha";

// El calendario trabaja con `Date` locales; el formulario, con "AAAA-MM-DD".
const aFecha = (texto: string) => {
  const [anio, mes, dia] = texto.split("-").map(Number);
  return new Date(anio, mes - 1, dia);
};
const aTexto = (fecha: Date) =>
  `${fecha.getFullYear()}-${String(fecha.getMonth() + 1).padStart(2, "0")}-${String(fecha.getDate()).padStart(2, "0")}`;

/** Campo de fecha con calendario desplegable (Popover de Radix, no modal: no
 * inyecta estilos y la CSP lo deja pasar). Con `rango` se eligen varios días:
 * primer toque el inicio, segundo el final. */
export default function SelectorFecha({
  rango,
  desde,
  hasta,
  minimo,
  error,
  onCambiar,
}: {
  rango: boolean;
  desde: string;
  hasta: string;
  minimo: string;
  error?: string;
  onCambiar: (desde: string, hasta: string) => void;
}) {
  const [abierto, setAbierto] = useState(false);
  const id = useId();
  const antes = { before: aFecha(minimo) };
  const comun = {
    locale: es,
    weekStartsOn: 1 as const,
    disabled: antes,
    startMonth: aFecha(minimo),
    defaultMonth: aFecha(desde),
  };

  return (
    <div className="campo" data-error={!!error}>
      <span id={`${id}-etiqueta`}>{rango ? "Días" : "Fecha"}</span>
      <Popover.Root open={abierto} onOpenChange={setAbierto}>
        <Popover.Trigger asChild>
          <button type="button" className="control-boton" aria-labelledby={`${id}-etiqueta ${id}-valor`}>
            <CalendarDays size={16} className="shrink-0 text-muted" aria-hidden="true" />
            <span id={`${id}-valor`} className="truncate">
              {rango ? rangoLegible(desde, hasta) : fechaCorta(desde)}
            </span>
          </button>
        </Popover.Trigger>
        <Popover.Portal>
          <Popover.Content className="flotante p-2" align="start" sideOffset={6} collisionPadding={12}>
            {rango ? (
              <DayPicker
                {...comun}
                mode="range"
                selected={{ from: aFecha(desde), to: aFecha(hasta) }}
                onSelect={(elegido) => {
                  if (!elegido?.from) return;
                  const inicio = aTexto(elegido.from);
                  const fin = elegido.to ? aTexto(elegido.to) : inicio;
                  onCambiar(inicio, fin);
                  if (fin > inicio) setAbierto(false);
                }}
              />
            ) : (
              <DayPicker
                {...comun}
                mode="single"
                required
                selected={aFecha(desde)}
                onSelect={(elegido) => {
                  const dia = aTexto(elegido);
                  onCambiar(dia, dia);
                  setAbierto(false);
                }}
              />
            )}
          </Popover.Content>
        </Popover.Portal>
      </Popover.Root>
      {error && <span className="campo-error">{error}</span>}
    </div>
  );
}
