import { Suspense, lazy, useState } from "react";
import type { ReactNode } from "react";
import { BadgeCheck, Boxes, HardHat, Mail } from "lucide-react";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import { SeccionActivaProvider, useSeccionActiva } from "../contexto/BarraEstadoContexto";
import { fechaHaceMeses } from "../tiempo";
import type { ClaseHistorial, PropsHistorialClase } from "./Historial.logica";

const HistorialContratistas = lazy(() => import("./HistorialContratistas"));
const HistorialProveedores = lazy(() => import("./HistorialProveedores"));
const HistorialCorreo = lazy(() => import("./HistorialCorreo"));
const HistorialKof = lazy(() => import("./HistorialKof"));

const OPCIONES_CLASE: OpcionSegmentada<ClaseHistorial>[] = [
  { valor: "contratistas", Icono: HardHat, titulo: "Contratistas" },
  { valor: "proveedores", Icono: Boxes, titulo: "Proveedores" },
  { valor: "correo", Icono: Mail, titulo: "Visitas por correo" },
  { valor: "kof", Icono: BadgeCheck, titulo: "Gafetes KOF" },
];

const VISTAS: Record<ClaseHistorial, (props: PropsHistorialClase) => ReactNode> = {
  contratistas: (props) => <HistorialContratistas {...props} />,
  proveedores: (props) => <HistorialProveedores {...props} />,
  correo: (props) => <HistorialCorreo {...props} />,
  kof: (props) => <HistorialKof {...props} />,
};

/** Capa de cada clase: las que ya se abrieron quedan montadas una encima de
 * la otra y sólo se muestra la elegida, así cambiar de clase no destruye la
 * grilla ni pierde scroll y filtros (mismo criterio que `.capa-vista` en
 * index.css, que usaban Proveedores y KOF para Activos/Historial). */
function claseCapaVista(visible: boolean): string {
  return visible ? "capa-vista" : "capa-vista capa-vista-oculta";
}

/**
 * Historial de todos los ingresos, con un selector de clase en el centro de
 * la barra. No es una lista mezclada: cada clase conserva su grilla, sus
 * columnas y su exportación, y el selector cambia de una a otra. El rango de
 * fechas es el mismo para todas (por defecto, los últimos 6 meses; `hasta`
 * vacío queda abierto, así no se pierden movimientos del día en curso).
 * Las visitas agendadas siguen en su propia pantalla.
 */
export default function Historial() {
  const seccionActiva = useSeccionActiva();
  const [clase, setClase] = useState<ClaseHistorial>("contratistas");
  // Cada clase se monta al abrirla por primera vez, no todas de entrada: la
  // de contratistas puede traer miles de filas.
  const [abiertas, setAbiertas] = useState<ClaseHistorial[]>(["contratistas"]);
  const [desde, setDesde] = useState(() => fechaHaceMeses(6));
  const [hasta, setHasta] = useState("");

  function elegirClase(nueva: ClaseHistorial) {
    setClase(nueva);
    setAbiertas((actuales) => (actuales.includes(nueva) ? actuales : [...actuales, nueva]));
  }

  const selector = (
    <SegmentadoOpciones
      opciones={OPCIONES_CLASE}
      valor={clase}
      onCambiar={elegirClase}
      etiqueta="Clase de ingreso"
    />
  );

  return (
    <div style={{ flex: 1, minHeight: 0, position: "relative", height: "100%" }}>
      {abiertas.map((claseAbierta) => (
        <div key={claseAbierta} className={claseCapaVista(claseAbierta === clase)}>
          {/* Sólo la clase visible publica su texto en la barra de estado. */}
          <SeccionActivaProvider value={seccionActiva && claseAbierta === clase}>
            <Suspense fallback={null}>
              {VISTAS[claseAbierta]({
                selector,
                desde,
                hasta,
                onRango: (nuevoDesde, nuevoHasta) => {
                  setDesde(nuevoDesde);
                  setHasta(nuevoHasta);
                },
              })}
            </Suspense>
          </SeccionActivaProvider>
        </div>
      ))}
    </div>
  );
}
