import { useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { BadgeCheck, Boxes, HardHat, Mail } from "lucide-react";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import { textoRangoFecha } from "../componentes/SelectorRangoFecha.logica";
import SelectorUnidadesOperativas from "../componentes/SelectorUnidadesOperativas";
import { textoUnidadesOperativas } from "../componentes/SelectorUnidadesOperativas.logica";
import { listarUnidadesOperativas } from "../api/historial";
import { fechaHaceMeses } from "../tiempo";
import { mensajeError } from "../mensajeError";
import HistorialContratistas from "./HistorialContratistas";
import HistorialClase from "./HistorialClase";
import type { ClaseHistorial, FiltroHistorial } from "./Historial.logica";

const OPCIONES_CLASE: OpcionSegmentada<ClaseHistorial>[] = [
  { valor: "contratistas", Icono: HardHat, titulo: "Contratistas" },
  { valor: "proveedores", Icono: Boxes, titulo: "Proveedores" },
  { valor: "correo", Icono: Mail, titulo: "Visitas por correo" },
  { valor: "kof", Icono: BadgeCheck, titulo: "Gafetes KOF" },
];

/**
 * Historial de todos los ingresos, con un selector de clase junto al buscador,
 * como en escritorio (`desktop/src/pantallas/Historial.tsx`): no es una lista
 * mezclada, cada clase conserva su grilla, sus columnas, su búsqueda y su
 * exportación. El rango de fechas (por defecto, los últimos 6 meses; `hasta`
 * vacío queda abierto, así no se pierden movimientos del día en curso) y las
 * unidades operativas son los mismos para todas.
 *
 * Cada clase se monta al abrirla por primera vez y después queda montada
 * (oculta cuando no se ve): cambiar de clase no pierde la página, el orden ni
 * los filtros de columna.
 */
export default function Historial() {
  const [clase, setClase] = useState<ClaseHistorial>("contratistas");
  const [abiertas, setAbiertas] = useState<ClaseHistorial[]>(["contratistas"]);
  const [desde, setDesde] = useState(() => fechaHaceMeses(6));
  const [hasta, setHasta] = useState("");
  // Guarda lo DESMARCADO, no lo marcado (mismo criterio que `ocultas` en
  // `Tabla.tsx`): una unidad que todavía no cargó, o una nueva que se dio de
  // alta después, nunca queda afuera del filtro por accidente.
  const [unidadesExcluidas, setUnidadesExcluidas] = useState<Set<string>>(new Set());

  const { data: unidades = [], error: errorUnidades } = useQuery({
    queryKey: ["historial", "unidades"],
    queryFn: listarUnidadesOperativas,
    staleTime: 5 * 60_000,
  });
  useEffect(() => {
    if (errorUnidades) toast.error(mensajeError(errorUnidades));
  }, [errorUnidades]);

  // `sitioIds` en `undefined` (sin filtro) cuando nada está excluido -- a
  // propósito distinto de "mandar la lista completa": así el primer render,
  // con `unidades` todavía vacío, no pide un `.in(sitio_id, [])` que traería
  // cero filas.
  const filtro = useMemo<FiltroHistorial>(
    () => ({
      desde: desde || undefined,
      hasta: hasta || undefined,
      sitioIds:
        unidadesExcluidas.size === 0
          ? undefined
          : unidades.filter((unidad) => !unidadesExcluidas.has(unidad.id)).map((unidad) => unidad.id),
    }),
    [desde, hasta, unidadesExcluidas, unidades],
  );
  const textoFiltro = `Filtro: ${textoRangoFecha(desde, hasta)} — ${textoUnidadesOperativas(unidades.length, unidadesExcluidas.size)}`;

  function elegirClase(nueva: ClaseHistorial) {
    setClase(nueva);
    setAbiertas((actuales) => (actuales.includes(nueva) ? actuales : [...actuales, nueva]));
  }

  const selector = (
    <SegmentadoOpciones opciones={OPCIONES_CLASE} valor={clase} onCambiar={elegirClase} etiqueta="Clase de ingreso" />
  );
  const compartidos = (
    <>
      <SelectorUnidadesOperativas unidades={unidades} excluidas={unidadesExcluidas} onCambiar={setUnidadesExcluidas} />
      <SelectorRangoFecha
        desde={desde}
        hasta={hasta}
        onAplicar={(nuevoDesde, nuevoHasta) => {
          setDesde(nuevoDesde);
          setHasta(nuevoHasta);
        }}
      />
    </>
  );

  return (
    <div className="h-full">
      {abiertas.map((abierta) => (
        <div key={abierta} className={abierta === clase ? "h-full" : "hidden"}>
          {abierta === "contratistas" ? (
            <HistorialContratistas selector={selector} compartidos={compartidos} filtro={filtro} textoFiltro={textoFiltro} />
          ) : (
            <HistorialClase
              clase={abierta}
              selector={selector}
              compartidos={compartidos}
              filtro={filtro}
              textoFiltro={textoFiltro}
            />
          )}
        </div>
      ))}
    </div>
  );
}
