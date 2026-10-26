import { useMemo, useState } from "react";
import type { ColDef } from "ag-grid-community";
import { BadgeCheck, Boxes, CircleOff, HardHat, IdCard, Layers, Mail, Users } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import Tabla from "../componentes/Tabla";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import { useLista } from "../componentes/useLista";
import { TEXTO_TIPO_ADENTRO, listarAdentroAhora, nivelPermanencia, textoTiempoAdentro } from "../api/adentro";
import type { PersonaAdentro, TipoAdentro } from "../api/adentro";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import {
  CLASES,
  ETIQUETA_CLASE,
  contarPorClase,
  contarPorGafete,
  filtrarPorClase,
  filtrarPorGafete,
} from "./Activos.logica";
import type { FiltroClase, FiltroGafete } from "./Activos.logica";

const ICONO_CLASE: Record<TipoAdentro, LucideIcon> = {
  CONTRATISTA: HardHat,
  PROVEEDOR: Boxes,
  POR_CORREO: Mail,
  PROVISIONAL_KOF: BadgeCheck,
};

/** Una opción por clase más "Todas", con la cantidad de cada una. Mientras
 * carga no hay números, para no enseñar ceros falsos. */
function opcionesFiltroClase(conteo: Record<TipoAdentro, number> | null): OpcionSegmentada<FiltroClase>[] {
  const todas = conteo ? CLASES.reduce((suma, clase) => suma + conteo[clase], 0) : undefined;
  return [
    {
      valor: "todas",
      Icono: Layers,
      titulo: todas === undefined ? "Todas las clases" : `${todas} en total`,
      cantidad: todas,
    },
    ...CLASES.map((clase) => ({
      valor: clase,
      Icono: ICONO_CLASE[clase],
      titulo: conteo ? `${ETIQUETA_CLASE[clase]}: ${conteo[clase]}` : ETIQUETA_CLASE[clase],
      cantidad: conteo?.[clase],
    })),
  ];
}

/** Todos / con gafete / sin gafete (S/G), con cuántos hay en cada uno. */
function opcionesFiltroGafete(conteo: { todos: number; con: number; sin: number } | null): OpcionSegmentada<FiltroGafete>[] {
  return [
    { valor: "todos", Icono: Users, titulo: conteo ? `${conteo.todos} adentro` : "Todos", cantidad: conteo?.todos },
    { valor: "con", Icono: IdCard, titulo: conteo ? `${conteo.con} con gafete` : "Con gafete", cantidad: conteo?.con },
    {
      valor: "sin",
      Icono: CircleOff,
      titulo: conteo ? `${conteo.sin} sin gafete (S/G)` : "Sin gafete (S/G)",
      cantidad: conteo?.sin,
    },
  ];
}

/** Resaltado de la fila según cuánto lleva adentro: la misma regla y los
 * mismos colores que escritorio (más de 12 h aviso, más de 14 h crítico).
 * Fuera del componente: `claseFila` tiene que ser una función estable. */
function claseFilaActivo(fila: PersonaAdentro): string | undefined {
  const nivel = nivelPermanencia(fila.hora_entrada);
  if (nivel === "critico") return "fila-permanencia-critico";
  if (nivel === "aviso") return "fila-permanencia-aviso";
  return undefined;
}

/**
 * Activos: quién está adentro ahora en cada unidad (contratistas, proveedores,
 * gafetes provisionales KOF e ingresos por correo sin salida). Sirve para el
 * control diario, para un conteo de emergencia y para ver salidas que se
 * olvidaron marcar. Sólo monitorea: las salidas se registran en las apps.
 *
 * Mismo nombre y mismos filtros que Activos de escritorio (2026-10-26): la
 * clase con su cantidad y, en contratistas, con o sin gafete; quien lleva más
 * de 12 h se marca en ámbar y más de 14 h en rojo.
 *
 * Se refresca con cada cambio de `ingresos` (Realtime) y cada 30 s: proveedores,
 * préstamos KOF e ingresos por correo no están en la publicación de Realtime,
 * así que dependen del intervalo.
 */
export default function Activos() {
  const [busqueda, setBusqueda] = useState("");
  const [filtroClase, setFiltroClase] = useState<FiltroClase>("todas");
  const [filtroGafete, setFiltroGafete] = useState<FiltroGafete>("todos");
  const { datos, cargando } = useLista(["adentro-ahora"], listarAdentroAhora, {
    intervaloMs: 30_000,
    tablas: "ingresos",
  });
  const filas = useMemo(() => datos ?? [], [datos]);

  const conteoClases = useMemo(() => (datos ? contarPorClase(filas) : null), [datos, filas]);
  const deClase = useMemo(() => filtrarPorClase(filas, filtroClase), [filas, filtroClase]);
  // Sólo los contratistas pueden entrar sin gafete: el filtro aparece con esa
  // clase elegida, como en escritorio.
  const mostrarFiltroGafete = filtroClase === "CONTRATISTA";
  const conteoGafete = useMemo(
    () => (datos ? { todos: deClase.length, ...contarPorGafete(deClase) } : null),
    [datos, deClase],
  );
  const filasVisibles = useMemo(
    () => filtrarPorGafete(deClase, mostrarFiltroGafete ? filtroGafete : "todos"),
    [deClase, mostrarFiltroGafete, filtroGafete],
  );

  const columnas = useMemo<ColDef<PersonaAdentro>[]>(
    () => [
      { field: "sitio_nombre", headerName: "Unidad", flex: 1.2, minWidth: 130 },
      {
        colId: "tipo",
        headerName: "Tipo",
        flex: 1.1,
        minWidth: 130,
        valueGetter: (p) => (p.data ? TEXTO_TIPO_ADENTRO[p.data.tipo] : ""),
      },
      // En KOF es el código de empleado del encargado.
      { field: "identificacion", headerName: "Cédula / código", flex: 1.2, minWidth: 140 },
      { field: "nombre", headerName: "Nombre", flex: 1.8, minWidth: 180 },
      { field: "empresa_nombre", headerName: "Empresa / motivo", flex: 1.3, minWidth: 140 },
      {
        field: "gafete_numero",
        headerName: "Gafete",
        flex: 0.8,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
      },
      { field: "placa", headerName: "Placa", flex: 0.9, minWidth: 100 },
      {
        colId: "entrada",
        headerName: "Entrada",
        flex: 1.3,
        minWidth: 150,
        // Ordena por el instante, muestra fecha y hora de Costa Rica.
        valueGetter: (p) => p.data?.hora_entrada ?? "",
        valueFormatter: (p) =>
          p.value ? `${textoFechaDDMMYYYY(fechaLocalYMD(p.value))} ${textoHora(p.value)}` : "",
        filter: false,
      },
      {
        colId: "tiempo",
        headerName: "Tiempo adentro",
        flex: 1.1,
        minWidth: 130,
        valueGetter: (p) => (p.data ? textoTiempoAdentro(p.data.hora_entrada) : ""),
        // Mismo orden que "Entrada": el texto ("3 h 20 min") no ordena bien.
        comparator: (_a, _b, nodoA, nodoB) =>
          (nodoB.data?.hora_entrada ?? "").localeCompare(nodoA.data?.hora_entrada ?? ""),
        filter: false,
      },
      { field: "usuario_entrada_nombre", headerName: "Dio ingreso", flex: 1.2, minWidth: 130 },
    ],
    [],
  );

  return (
    <div className="flex h-full flex-col">
      <div className="pantalla-cuerpo min-h-0 flex-1">
        <div className="min-h-0 flex-1">
          <Tabla<PersonaAdentro>
            // El id de siempre: así nadie pierde el orden y ancho de columnas
            // que ya había acomodado cuando la pantalla se llamaba "Adentro ahora".
            id="adentro-ahora"
            columnas={columnas}
            filas={filasVisibles}
            busqueda={busqueda}
            filtrosPorColumna
            idFila={(fila) => `${fila.tipo}:${fila.id}`}
            claseFila={claseFilaActivo}
            controles={
              <>
                {/* Primero el buscador y después los filtros (pedido del
                    usuario 2026-10-26). */}
                <div className="campo flex-[0_1_16rem]">
                  <input
                    placeholder="Cédula, nombre, gafete…"
                    value={busqueda}
                    disabled={cargando}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
                <SegmentadoOpciones
                  opciones={opcionesFiltroClase(conteoClases)}
                  valor={filtroClase}
                  onCambiar={setFiltroClase}
                  etiqueta="Filtrar por clase"
                />
                {mostrarFiltroGafete && (
                  <SegmentadoOpciones
                    opciones={opcionesFiltroGafete(conteoGafete)}
                    valor={filtroGafete}
                    onCambiar={setFiltroGafete}
                    etiqueta="Filtrar por gafete"
                  />
                )}
              </>
            }
          />
        </div>
      </div>
    </div>
  );
}
