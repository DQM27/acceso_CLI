import { useEffect, useRef, useState } from "react";
import { CalendarDays } from "lucide-react";
import { ListaFlotante, useListaFlotante } from "./ListaFlotante";
import { fechaYMD, textoFechaDDMMYYYY } from "../tiempo";

/**
 * Botón "Período: ..." que abre un popover con accesos rápidos (Hoy, Esta
 * semana, etc.) + los dos campos de fecha para un rango custom. Copiado de
 * `desktop/src/componentes/SelectorRangoFecha.tsx`.
 *
 * Los cambios quedan en un borrador local (`desdeBorrador`/`hastaBorrador`)
 * hasta "Aplicar" — clickear un preset o tipear en los campos no dispara
 * `onAplicar` todavía, así el usuario puede tocar varias cosas antes de
 * confirmar (o "Cancelar" y no cambiar nada).
 */

/** Mismo texto que muestra el botón "Período: ..." — se exporta para que
 * quien necesite describir el filtro activo en otro lado no reimplemente
 * este formateo. `desde`/`hasta` vacíos son extremos abiertos; se describe
 * cada combinación en vez de asumir que "falta uno" significa "sin
 * filtro". */
export function textoRangoFecha(desde: string, hasta: string): string {
  if (desde && hasta) return `${textoFechaDDMMYYYY(desde)} – ${textoFechaDDMMYYYY(hasta)}`;
  if (desde) return `Desde ${textoFechaDDMMYYYY(desde)}`;
  if (hasta) return `Hasta ${textoFechaDDMMYYYY(hasta)}`;
  return "Todo el historial";
}

export interface Preset {
  etiqueta: string;
  calcular: (hoy: Date) => { desde: string; hasta: string };
}

/** Lunes de la semana que contiene `d` — la semana arranca en lunes acá
 * (convención de semana laboral), no domingo. */
function inicioSemana(d: Date): Date {
  const dia = d.getDay(); // 0 = domingo … 6 = sábado
  const offset = dia === 0 ? 6 : dia - 1;
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() - offset);
}

export const PRESETS: Preset[] = [
  {
    etiqueta: "Hoy",
    calcular: (hoy) => ({ desde: fechaYMD(hoy), hasta: fechaYMD(hoy) }),
  },
  {
    etiqueta: "Ayer",
    calcular: (hoy) => {
      const ayer = new Date(hoy.getFullYear(), hoy.getMonth(), hoy.getDate() - 1);
      return { desde: fechaYMD(ayer), hasta: fechaYMD(ayer) };
    },
  },
  {
    etiqueta: "Esta semana",
    calcular: (hoy) => ({ desde: fechaYMD(inicioSemana(hoy)), hasta: fechaYMD(hoy) }),
  },
  {
    etiqueta: "Semana pasada",
    calcular: (hoy) => {
      const inicioActual = inicioSemana(hoy);
      const inicioPasada = new Date(
        inicioActual.getFullYear(),
        inicioActual.getMonth(),
        inicioActual.getDate() - 7,
      );
      const finPasada = new Date(
        inicioActual.getFullYear(),
        inicioActual.getMonth(),
        inicioActual.getDate() - 1,
      );
      return { desde: fechaYMD(inicioPasada), hasta: fechaYMD(finPasada) };
    },
  },
  {
    etiqueta: "Este mes",
    calcular: (hoy) => ({
      desde: fechaYMD(new Date(hoy.getFullYear(), hoy.getMonth(), 1)),
      hasta: fechaYMD(hoy),
    }),
  },
  {
    etiqueta: "Mes pasado",
    calcular: (hoy) => ({
      desde: fechaYMD(new Date(hoy.getFullYear(), hoy.getMonth() - 1, 1)),
      // Día 0 del mes actual == último día del mes anterior.
      hasta: fechaYMD(new Date(hoy.getFullYear(), hoy.getMonth(), 0)),
    }),
  },
  {
    etiqueta: "Últimos 7 días",
    calcular: (hoy) => ({
      desde: fechaYMD(new Date(hoy.getFullYear(), hoy.getMonth(), hoy.getDate() - 6)),
      hasta: fechaYMD(hoy),
    }),
  },
  {
    etiqueta: "Últimos 30 días",
    calcular: (hoy) => ({
      desde: fechaYMD(new Date(hoy.getFullYear(), hoy.getMonth(), hoy.getDate() - 29)),
      hasta: fechaYMD(hoy),
    }),
  },
];

export default function SelectorRangoFecha({
  desde,
  hasta,
  onAplicar,
}: {
  desde: string;
  hasta: string;
  onAplicar: (desde: string, hasta: string) => void;
}) {
  const [abierto, setAbierto] = useState(false);
  const [desdeBorrador, setDesdeBorrador] = useState(desde);
  const [hastaBorrador, setHastaBorrador] = useState(hasta);
  const { campoRef, posicion } = useListaFlotante(abierto);
  const popoverRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!abierto) return;
    function alHacerClicAfuera(evento: MouseEvent) {
      const objetivo = evento.target as Node;
      if (campoRef.current?.contains(objetivo) || popoverRef.current?.contains(objetivo)) return;
      setAbierto(false);
    }
    document.addEventListener("mousedown", alHacerClicAfuera);
    return () => document.removeEventListener("mousedown", alHacerClicAfuera);
  }, [abierto, campoRef]);

  function abrir() {
    setDesdeBorrador(desde);
    setHastaBorrador(hasta);
    setAbierto(true);
  }

  function aplicar() {
    onAplicar(desdeBorrador, hastaBorrador);
    setAbierto(false);
  }

  const etiqueta = textoRangoFecha(desde, hasta);

  return (
    <>
      <div ref={campoRef}>
        <button type="button" className="boton boton-icono" onClick={abrir}>
          <CalendarDays size={16} />
          Período: {etiqueta}
        </button>
      </div>
      {abierto && posicion && (
        <ListaFlotante posicion={posicion} ancho={280}>
          <div
            ref={popoverRef}
            style={{
              padding: "0.9rem",
              display: "flex",
              flexDirection: "column",
              gap: "0.75rem",
            }}
          >
            <div>
              <p
                style={{
                  margin: "0 0 0.4rem",
                  fontSize: "0.75rem",
                  color: "var(--muted)",
                  textTransform: "uppercase",
                  letterSpacing: "0.04em",
                }}
              >
                Acceso rápido
              </p>
              <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "0.4rem" }}>
                {PRESETS.map((preset) => (
                  <button
                    key={preset.etiqueta}
                    type="button"
                    className="boton"
                    style={{ padding: "0.4rem 0.6rem", fontSize: "0.85rem" }}
                    onClick={() => {
                      const rango = preset.calcular(new Date());
                      setDesdeBorrador(rango.desde);
                      setHastaBorrador(rango.hasta);
                    }}
                  >
                    {preset.etiqueta}
                  </button>
                ))}
              </div>
            </div>
            <label className="campo">
              Desde
              <input
                type="date"
                value={desdeBorrador}
                max={hastaBorrador || undefined}
                onChange={(e) => setDesdeBorrador(e.target.value)}
              />
            </label>
            <label className="campo">
              Hasta
              <input
                type="date"
                value={hastaBorrador}
                min={desdeBorrador || undefined}
                onChange={(e) => setHastaBorrador(e.target.value)}
              />
            </label>
            <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
              <button type="button" className="boton" onClick={() => setAbierto(false)}>
                Cancelar
              </button>
              <button type="button" className="boton boton-primario" onClick={aplicar}>
                Aplicar
              </button>
            </div>
          </div>
        </ListaFlotante>
      )}
    </>
  );
}
