import { useEffect, useRef, useState } from "react";
import { ImageDown } from "lucide-react";
import { descargarArchivo } from "../exportacion";
import { CATALOGO, type Origen } from "./catalogo";
import { nombreDeArchivo, pngDesdeDataUrl } from "./imagen";
import type { Fila } from "./calculo";
import type { WidgetLibre } from "./esquema";

/**
 * Panel "libre" del lienzo: un dibujo a medida que escribió el asistente.
 *
 * El código NO corre en el panel: va a un iframe en otro dominio
 * (`lienzo.megabrisas.com`, carpeta `sandbox-lienzo/`) con
 * sandbox="allow-scripts" (origen anónimo: no ve la sesión, las cookies ni esta
 * página). Ahí corre en un Web Worker sin red y con tiempo límite, y lo que
 * devuelve se limpia antes de mostrarse. De vuelta sólo llegan el alto del
 * dibujo y, si falló, un mensaje.
 */
export const URL_SANDBOX: string = import.meta.env.VITE_LIENZO_SANDBOX_URL || "https://lienzo.megabrisas.com/";

/** Si el sandbox no avisa que cargó en este tiempo, se muestra el problema en vez de un hueco. */
const ESPERA_CARGA_MS = 10_000;
const ALTO_INICIAL = 260;

interface ColumnaParaDibujo {
  nombre: string;
  tipo: string;
  etiqueta: string;
}

function columnasDe(origen: Origen): ColumnaParaDibujo[] {
  if (typeof origen !== "string") return origen.columnas.map((c) => ({ nombre: c.nombre, tipo: c.tipo, etiqueta: c.etiqueta ?? c.nombre }));
  return Object.entries(CATALOGO[origen].campos).map(([nombre, def]) => ({ nombre, tipo: def.tipo, etiqueta: def.etiqueta }));
}

const HEX = /^#[0-9a-f]{6}$/i;

/** Colores del tema actual (claro u oscuro), leídos de las variables del lienzo. */
function temaActual(elemento: Element | null) {
  const estilo = getComputedStyle(elemento ?? document.documentElement);
  const leer = (variable: string) => {
    const valor = estilo.getPropertyValue(variable).trim();
    return HEX.test(valor) ? valor : undefined;
  };
  const colores = Array.from({ length: 8 }, (_, i) => leer(`--serie-${i + 1}`)).filter((c): c is string => Boolean(c));
  return {
    colores,
    otros: leer("--serie-otros"),
    intenso: leer("--serie-unica") ?? colores[0],
    fondo: leer("--panel"),
    texto: leer("--texto"),
    textoSuave: leer("--muted"),
    rejilla: leer("--borde"),
    tipFondo: leer("--texto"),
    tipTexto: leer("--panel"),
  };
}

export default function PanelLibre({ widget, filas, origen }: { widget: WidgetLibre; filas: Fila[]; origen: Origen }) {
  const marco = useRef<HTMLIFrameElement>(null);
  // Para nombrar el PNG cuando vuelve del sandbox (el oyente se registra una sola vez).
  const tituloActual = useRef(widget.titulo);
  useEffect(() => {
    tituloActual.current = widget.titulo;
  }, [widget.titulo]);
  const [preparado, setPreparado] = useState(false);
  const [alto, setAlto] = useState(ALTO_INICIAL);
  const [error, setError] = useState<string | null>(null);
  const [sinSandbox, setSinSandbox] = useState(false);

  // Sólo se escucha a este marco: cualquier otro mensaje se ignora.
  useEffect(() => {
    const alRecibir = (evento: MessageEvent) => {
      if (!marco.current || evento.source !== marco.current.contentWindow) return;
      const m = evento.data as { tipo?: string; alto?: unknown; mensaje?: unknown };
      if (m?.tipo === "preparado") setPreparado(true);
      else if (m?.tipo === "alto" && typeof m.alto === "number") setAlto(Math.min(1600, Math.max(60, Math.ceil(m.alto))));
      else if (m?.tipo === "listo") setError(null);
      else if (m?.tipo === "error") setError(typeof m.mensaje === "string" ? m.mensaje.slice(0, 300) : "Error desconocido");
      else if (m?.tipo === "png") {
        const png = pngDesdeDataUrl((evento.data as { datos?: unknown }).datos);
        if (png) descargarArchivo(nombreDeArchivo(tituloActual.current, "png"), png, "image/png");
        else setError("La imagen que devolvió el espacio de dibujo no es válida.");
      } else if (m?.tipo === "png_error") setError(typeof m.mensaje === "string" ? m.mensaje.slice(0, 300) : "No se pudo exportar");
    };
    window.addEventListener("message", alRecibir);
    return () => window.removeEventListener("message", alRecibir);
  }, []);

  useEffect(() => {
    if (preparado) return;
    const espera = setTimeout(() => setSinSandbox(true), ESPERA_CARGA_MS);
    return () => clearTimeout(espera);
  }, [preparado]);

  // Cada vez que cambian el código o las filas (un filtro, Deshacer), se vuelve a dibujar.
  useEffect(() => {
    const destino = marco.current?.contentWindow;
    if (!preparado || !destino) return;
    // "*": el marco tiene origen anónimo y no hay otro destino posible; sólo esa ventana lo recibe.
    destino.postMessage(
      { tipo: "dibujar", id: Date.now(), codigo: widget.codigo, datos: filas, columnas: columnasDe(origen), tema: temaActual(marco.current) },
      "*",
    );
  }, [preparado, widget.codigo, filas, origen]);

  return (
    <section className="tarjeta lienzo-bloque lienzo-libre">
      <header className="lienzo-bloque-cabecera">
        <h3>{widget.titulo}</h3>
        <span className="lienzo-libre-marca" title="Dibujo armado por el asistente, en un espacio aislado">
          A medida
        </span>
        <button
          type="button"
          className="lienzo-alternar"
          disabled={!preparado}
          title="Descargar como imagen (PNG)"
          aria-label={`Descargar ${widget.titulo} como imagen`}
          onClick={() => marco.current?.contentWindow?.postMessage({ tipo: "exportar_png", titulo: widget.titulo }, "*")}
        >
          <ImageDown size={16} aria-hidden="true" />
        </button>
      </header>
      {widget.descripcion && <p className="lienzo-libre-descripcion">{widget.descripcion}</p>}
      {sinSandbox && !preparado ? (
        <p className="asistente-error" role="alert">
          No se pudo cargar el espacio de dibujo ({new URL(URL_SANDBOX).host}).
        </p>
      ) : (
        <iframe
          ref={marco}
          src={URL_SANDBOX}
          title={widget.descripcion ?? widget.titulo}
          sandbox="allow-scripts"
          referrerPolicy="no-referrer"
          loading="lazy"
          className="lienzo-libre-marco"
          style={{ height: alto }}
        />
      )}
      {error && (
        <p className="asistente-error" role="alert">
          El dibujo falló: {error}
        </p>
      )}
    </section>
  );
}
