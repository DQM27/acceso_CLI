import { useEffect, useRef, useState } from "react";
import { CalendarDays } from "lucide-react";
import { ListaFlotante } from "./ListaFlotante";
import { useListaFlotante } from "./ListaFlotante.logica";
import { textoRangoFecha, PRESETS, etiquetaCortaRango } from "./SelectorRangoFecha.logica";

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

  // El mismo botón abre y cierra (antes sólo abría; para cerrar había que
  // hacer clic afuera -- pedido del usuario 2026-09-23). El clic-afuera de
  // arriba ignora a propósito los clics sobre el botón, así que no chocan.
  function alternar() {
    if (abierto) {
      setAbierto(false);
    } else {
      abrir();
    }
  }

  function aplicar() {
    onAplicar(desdeBorrador, hastaBorrador);
    setAbierto(false);
  }

  return (
    <>
      <div ref={campoRef}>
        {/* Compacto: el ícono ya dice "período", así que sólo va el rango
            corto (ver `etiquetaCortaRango`); el texto completo queda en el
            título. En mayúsculas (pedido del usuario 2026-09-23) -- va en el
            propio botón porque los botones no heredan `text-transform`. */}
        <button
          type="button"
          className="boton boton-icono"
          onClick={alternar}
          aria-expanded={abierto}
          title={`Período: ${textoRangoFecha(desde, hasta)}`}
          style={{ textTransform: "uppercase", fontWeight: 400 }}
        >
          <CalendarDays size={16} />
          {etiquetaCortaRango(desde, hasta)}
        </button>
      </div>
      {/* Se abre al costado izquierdo del botón, a su misma altura (pedido
          del usuario 2026-09-23), así nunca tapa los botones de al lado
          (Excel/CSV/PDF). Ancho fijo: lo que piden los accesos rápidos en
          dos columnas, cada uno en una sola línea. */}
      {abierto && posicion && (
        <ListaFlotante posicion={posicion} ancho={260} direccion="izquierda">
          <div
            ref={popoverRef}
            className="flex flex-col gap-3 p-[0.9rem]"
          >
            <div>
              <p
                className="m-0 mb-[0.4rem] text-[0.75rem] uppercase tracking-[0.04em] text-muted"
              >
                Acceso rápido
              </p>
              <div className="grid grid-cols-2 gap-[0.4rem]">
                {PRESETS.map((preset) => (
                  <button
                    key={preset.etiqueta}
                    title={preset.etiqueta}
                    type="button"
                    className="boton"
                    style={{
                      padding: "0.35rem 0.4rem",
                      fontSize: "0.78rem",
                      // Sin negrita y sin saltos: cada acceso rápido en una
                      // sola línea (en negrita "SEMANA PASADA" y "ÚLTIMOS 30
                      // DÍAS" se partían en dos).
                      fontWeight: 400,
                      whiteSpace: "nowrap",
                      // En mayúsculas, como "ACCESO RÁPIDO".
                      textTransform: "uppercase",
                    }}
                    // Un acceso rápido aplica y cierra en el acto (pedido del
                    // usuario 2026-09-23: elegir y después "Aplicar" era un
                    // paso de más). "Aplicar" queda para el rango a mano.
                    onClick={() => {
                      const rango = preset.calcular(new Date());
                      onAplicar(rango.desde, rango.hasta);
                      setAbierto(false);
                    }}
                  >
                    {preset.corta}
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
            <div className="flex justify-end gap-2">
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
