import { useCallback, useMemo, useState } from "react";
import { toast } from "sonner";
import { CircleDashed, FlaskConical, History, Layers, MonitorSmartphone, RotateCcw, TriangleAlert } from "lucide-react";
import type { ColDef } from "ag-grid-community";
import Modal from "../componentes/Modal";
import Tabla from "../componentes/Tabla";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import { useLista } from "../componentes/useLista";
import { listarDispositivosYSitios } from "../api/dispositivos";
import type { Dispositivo, Sitio } from "../api/dispositivos";
import {
  GRUPOS_POR_DEFECTO,
  GRUPOS_TELEMETRIA,
  LARGO_MAXIMO_NOTA,
  guardarOrdenTelemetria,
  listarCambiosTelemetria,
  listarOrdenesTelemetria,
} from "../api/telemetria";
import type { CambioOrdenTelemetria, GrupoTelemetria, OrdenTelemetria } from "../api/telemetria";
import type { EstadoEstudio } from "./Telemetria.logica";
import {
  DESCRIPCION_GRUPO,
  OPCIONES_VENCIMIENTO,
  contarPorEstado,
  esLoDeSiempre,
  estadoEstudio,
  gruposVigentes,
  ordenVencida,
  ordenarGrupos,
  textoGrupos,
  textoVence,
  venceEnDesde,
} from "./Telemetria.logica";
import { mensajeError } from "../mensajeError";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

interface DatosTelemetria {
  sitios: Sitio[];
  /** Vigentes y visibles en el panel, de todos los tipos. */
  equipos: Dispositivo[];
  ordenes: Map<string, OrdenTelemetria>;
  /** Contra esta hora se decide qué orden ya venció. */
  cargadoEn: Date;
}

async function cargarDatos(): Promise<DatosTelemetria> {
  const [estado, ordenes] = await Promise.all([listarDispositivosYSitios(), listarOrdenesTelemetria()]);
  return {
    sitios: [...estado.sitios].sort((a, b) => a.nombre.localeCompare(b.nombre, "es")),
    equipos: estado.dispositivos
      .filter((equipo) => !equipo.revoked_at && !equipo.oculto_en_panel)
      .sort((a, b) => a.etiqueta.localeCompare(b.etiqueta, "es")),
    ordenes,
    cargadoEn: new Date(),
  };
}

/** "18/10/2026 14:32", en la hora del navegador. */
function textoFecha(iso: string): string {
  return `${textoFechaDDMMYYYY(fechaLocalYMD(iso))} ${textoHora(iso)}`;
}

const TEXTO_TIPO: Record<Dispositivo["tipo"], string> = { pc: "PC", mobile: "Celular", visor: "Visor" };

/** Una fila de la grilla: el equipo y lo que mide ahora. */
interface FilaTelemetria {
  equipo: Dispositivo;
  unidad: string;
  orden: OrdenTelemetria | undefined;
  estado: EstadoEstudio;
  grupos: readonly GrupoTelemetria[];
}

type FiltroEstado = "todos" | EstadoEstudio;

const TEXTO_ESTADO: Record<EstadoEstudio, string> = { con_orden: "Con orden", siempre: "Lo de siempre" };

function opcionesFiltro(conteo: Record<EstadoEstudio, number> | null): OpcionSegmentada<FiltroEstado>[] {
  const total = conteo ? conteo.con_orden + conteo.siempre : undefined;
  return [
    { valor: "todos", Icono: Layers, titulo: "Todos los equipos", cantidad: total },
    { valor: "con_orden", Icono: FlaskConical, titulo: "Con una orden de estudios", cantidad: conteo?.con_orden },
    { valor: "siempre", Icono: CircleDashed, titulo: "Midiendo lo de siempre", cantidad: conteo?.siempre },
  ];
}

/** Color del punto de la columna "Estado": la depuración cuesta, se distingue. */
function colorEstado(fila: FilaTelemetria): string {
  if (fila.estado === "siempre") return "var(--muted)";
  return fila.grupos.includes("depuracion") ? "var(--advertencia)" : "var(--exito)";
}

/** Formulario de la orden de un equipo. Arranca con lo que mide hoy. */
function EditarOrden({
  equipo,
  orden,
  ahora,
  onCerrar,
  onGuardada,
}: {
  equipo: Dispositivo;
  orden: OrdenTelemetria | undefined;
  ahora: Date;
  onCerrar: () => void;
  onGuardada: () => void;
}) {
  const vigente = orden !== undefined && !ordenVencida(orden, ahora);
  // Sin orden o con la orden vencida, el equipo ya mide lo de siempre.
  const yaEsLoDeSiempre = !vigente || (esLoDeSiempre(orden.grupos) && orden.vence_en === null);
  const [marcados, setMarcados] = useState<Set<GrupoTelemetria>>(() => new Set(gruposVigentes(orden, ahora)));
  // "mantener" sólo si la orden vigente tiene vencimiento: conservarlo en vez
  // de alargarlo sin querer al cambiar un grupo.
  const puedeMantener = vigente && orden.vence_en !== null;
  const [vencimiento, setVencimiento] = useState<string>(puedeMantener ? "mantener" : "168");
  const [nota, setNota] = useState(vigente ? orden.nota : "");
  const [guardando, setGuardando] = useState(false);

  const grupos = ordenarGrupos(marcados, GRUPOS_TELEMETRIA);
  const horas = vencimiento === "mantener" || vencimiento === "nunca" ? null : Number(vencimiento);
  const sinVencimiento = vencimiento === "nunca";

  async function guardar(datos: { grupos: readonly GrupoTelemetria[]; vence_en: string | null; nota: string }) {
    setGuardando(true);
    try {
      await guardarOrdenTelemetria({ dispositivo_id: equipo.id, ...datos });
      toast.success(`Orden guardada para ${equipo.etiqueta}: ${textoGrupos(datos.grupos)}.`);
      onGuardada();
    } catch (fallo) {
      toast.error(mensajeError(fallo));
      setGuardando(false);
    }
  }

  function alGuardar() {
    const venceEn = vencimiento === "mantener" && orden ? orden.vence_en : venceEnDesde(horas, new Date());
    void guardar({ grupos, vence_en: venceEn, nota: nota.trim() });
  }

  function alternar(grupo: GrupoTelemetria) {
    setMarcados((actual) => {
      const siguiente = new Set(actual);
      if (siguiente.has(grupo)) siguiente.delete(grupo);
      else siguiente.add(grupo);
      return siguiente;
    });
  }

  return (
    <Modal titulo={`Telemetría de ${equipo.etiqueta}`} onCerrar={() => !guardando && onCerrar()}>
      <div className="flex flex-col gap-3">
        <fieldset className="flex flex-col gap-2">
          <legend className="mb-1 text-sm text-(--muted)">
            Estudios (base va siempre: equipos, versiones y caídas)
          </legend>
          {GRUPOS_TELEMETRIA.map((grupo) => (
            <label key={grupo} className="flex items-start gap-2">
              <input
                type="checkbox"
                className="mt-1"
                checked={marcados.has(grupo)}
                disabled={guardando}
                onChange={() => alternar(grupo)}
              />
              <span className="flex flex-col">
                <span className="font-medium">{DESCRIPCION_GRUPO[grupo].split(": ")[0]}</span>
                <span className="text-sm text-(--muted)">{DESCRIPCION_GRUPO[grupo].split(": ")[1]}</span>
              </span>
            </label>
          ))}
        </fieldset>

        <label className="campo">
          Vence en
          <select value={vencimiento} disabled={guardando} onChange={(evento) => setVencimiento(evento.target.value)}>
            {puedeMantener && orden.vence_en && (
              <option value="mantener">Mantener (hasta el {textoFecha(orden.vence_en)})</option>
            )}
            {OPCIONES_VENCIMIENTO.map((opcion) => (
              <option key={opcion.etiqueta} value={opcion.horas === null ? "nunca" : String(opcion.horas)}>
                {opcion.etiqueta}
              </option>
            ))}
          </select>
        </label>

        {marcados.has("depuracion") && sinVencimiento && (
          <p className="flex items-center gap-2 text-sm text-error">
            <TriangleAlert size={16} aria-hidden="true" />
            Depuración sin vencimiento hace más lento al equipo hasta que alguien la apague.
          </p>
        )}

        <label className="campo">
          <span className="flex justify-between">
            <span>Nota</span>
            <span aria-live="polite">
              {nota.length}/{LARGO_MAXIMO_NOTA}
            </span>
          </span>
          <input
            value={nota}
            maxLength={LARGO_MAXIMO_NOTA}
            disabled={guardando}
            placeholder="Ej.: el cliente dice que se traba al escanear"
            onChange={(evento) => setNota(evento.target.value)}
          />
        </label>

        <p className="text-sm text-(--muted)">
          Al vencer, el equipo vuelve solo a lo de siempre ({textoGrupos(GRUPOS_POR_DEFECTO)}), aunque no tenga red.
        </p>

        <div className="flex flex-wrap justify-between gap-2">
          <button
            type="button"
            className="boton boton-icono"
            disabled={guardando || yaEsLoDeSiempre}
            title="Lo que mide un equipo sin orden, sin vencimiento"
            onClick={() => void guardar({ grupos: GRUPOS_POR_DEFECTO, vence_en: null, nota: "" })}
          >
            <RotateCcw size={16} aria-hidden="true" />
            Volver a lo de siempre
          </button>
          <div className="flex gap-2">
            <button type="button" className="boton" disabled={guardando} onClick={onCerrar}>
              Cancelar
            </button>
            <button type="button" className="boton boton-primario" disabled={guardando} onClick={alGuardar}>
              {guardando ? "Guardando…" : "Guardar"}
            </button>
          </div>
        </div>
      </div>
    </Modal>
  );
}

type Vista = "equipos" | "cambios";

const OPCIONES_VISTA: OpcionSegmentada<Vista>[] = [
  { valor: "equipos", Icono: MonitorSmartphone, titulo: "Equipos: qué mide cada uno" },
  { valor: "cambios", Icono: History, titulo: "Cambios: quién cambió qué orden y cuándo" },
];

/** Quién cambió qué y cuándo: para cruzar un cambio en los datos con la orden
 * que lo causó. */
const COLUMNAS_CAMBIOS: ColDef<CambioOrdenTelemetria>[] = [
  {
    colId: "cuando",
    headerName: "Cuándo",
    flex: 1.1,
    minWidth: 150,
    sort: "desc",
    valueGetter: ({ data }) => (data ? textoFecha(data.cambiado_en) : ""),
    comparator: (_a, _b, nodoA, nodoB) =>
      (nodoA.data?.cambiado_en ?? "").localeCompare(nodoB.data?.cambiado_en ?? ""),
    filter: false,
  },
  {
    colId: "equipo",
    headerName: "Equipo",
    flex: 1.4,
    minWidth: 170,
    valueGetter: ({ data }) => data?.etiqueta ?? "Equipo que ya no existe",
    cellStyle: { textAlign: "left" },
  },
  {
    colId: "estudios",
    headerName: "Estudios",
    flex: 1.6,
    minWidth: 200,
    valueGetter: ({ data }) => (data ? textoGrupos(data.grupos) : ""),
    cellStyle: { textAlign: "left" },
  },
  {
    colId: "vence",
    headerName: "Vence",
    flex: 1.1,
    minWidth: 150,
    valueGetter: ({ data }) => (data?.vence_en ? textoFecha(data.vence_en) : "Sin vencimiento"),
    comparator: (_a, _b, nodoA, nodoB) =>
      (nodoA.data?.vence_en ?? "9999").localeCompare(nodoB.data?.vence_en ?? "9999"),
    filter: false,
  },
  { colId: "version", headerName: "Versión", flex: 0.6, minWidth: 90, valueGetter: ({ data }) => data?.version },
  {
    colId: "quien",
    headerName: "Quién",
    flex: 1.3,
    minWidth: 170,
    valueGetter: ({ data }) => data?.cambiado_por,
    cellStyle: { textAlign: "left" },
  },
  {
    colId: "nota",
    headerName: "Nota",
    flex: 1.4,
    minWidth: 160,
    valueGetter: ({ data }) => data?.nota || "—",
    cellStyle: { textAlign: "left" },
  },
];

/**
 * Estudios de telemetría por equipo (fase 3 del plan de telemetría). El
 * administrador enciende y apaga grupos de mediciones en cada equipo, con
 * vencimiento y nota. La nube le avisa al equipo al instante por Realtime, y
 * el equipo además la lee al sincronizar si estaba apagado o sin red.
 *
 * Qué versión de la orden aplicó cada equipo no se ve acá: lo informa el
 * propio equipo en su evento `orden_telemetria`, que va a la telemetría de
 * staging (Grafana), no a esta base.
 */
export default function Telemetria() {
  // Sin Realtime: las órdenes sólo las escribe este panel, que recarga al guardar.
  const { datos, cargando, recargar } = useLista(["telemetria-equipos"], cargarDatos, {
    intervaloMs: 60_000,
    tablas: "",
  });
  const historial = useLista(["telemetria-cambios"], listarCambiosTelemetria, { intervaloMs: 60_000, tablas: "" });
  const [editando, setEditando] = useState<string | null>(null);
  const [busqueda, setBusqueda] = useState("");
  const [filtro, setFiltro] = useState<FiltroEstado>("todos");
  const [vista, setVista] = useState<Vista>("equipos");

  const equipoEditado = datos?.equipos.find((equipo) => equipo.id === editando);
  const filas = useMemo<FilaTelemetria[]>(() => {
    if (!datos) return [];
    const nombreSitio = new Map(datos.sitios.map((sitio) => [sitio.id, sitio.nombre]));
    return datos.equipos.map((equipo) => {
      const orden = datos.ordenes.get(equipo.id);
      return {
        equipo,
        unidad: nombreSitio.get(equipo.sitio_id) ?? "—",
        orden,
        estado: estadoEstudio(orden, datos.cargadoEn),
        grupos: gruposVigentes(orden, datos.cargadoEn),
      };
    });
  }, [datos]);
  const conteo = datos ? contarPorEstado(filas.map((fila) => fila.estado)) : null;
  const filasVisibles = filtro === "todos" ? filas : filas.filter((fila) => fila.estado === filtro);

  const cambiar = useCallback((fila: FilaTelemetria) => setEditando(fila.equipo.id), []);
  const ahora = datos?.cargadoEn;

  const columnas = useMemo<ColDef<FilaTelemetria>[]>(
    () => [
      { colId: "unidad", headerName: "Unidad", flex: 1, minWidth: 130, valueGetter: ({ data }) => data?.unidad },
      {
        colId: "equipo",
        headerName: "Equipo",
        flex: 1.6,
        minWidth: 190,
        valueGetter: ({ data }) => data?.equipo.etiqueta,
        cellStyle: { textAlign: "left" },
      },
      {
        colId: "tipo",
        headerName: "Tipo",
        flex: 0.7,
        minWidth: 100,
        valueGetter: ({ data }) => (data ? TEXTO_TIPO[data.equipo.tipo] : ""),
      },
      {
        colId: "estado",
        headerName: "Estado",
        flex: 0.9,
        minWidth: 130,
        valueGetter: ({ data }) => (data ? TEXTO_ESTADO[data.estado] : ""),
        cellRenderer: ({ data }: { data: FilaTelemetria }) => (
          <span className="chip" style={{ ["--chip-color" as string]: colorEstado(data) }}>
            {TEXTO_ESTADO[data.estado]}
          </span>
        ),
      },
      {
        colId: "estudios",
        headerName: "Estudios",
        flex: 1.6,
        minWidth: 200,
        valueGetter: ({ data }) => (data ? textoGrupos(data.grupos) : ""),
        cellStyle: { textAlign: "left" },
      },
      {
        colId: "vence",
        headerName: "Vence",
        flex: 1.3,
        minWidth: 170,
        filter: false,
        valueGetter: ({ data }) => (data && ahora ? textoVence(data.orden, ahora, textoFecha) : ""),
        // Por el instante, no por el texto; sin vencimiento al final.
        comparator: (_a, _b, nodoA, nodoB) => {
          const instante = (fila: FilaTelemetria | undefined) =>
            fila?.orden?.vence_en ?? (fila?.orden ? "9999" : "");
          return instante(nodoA.data).localeCompare(instante(nodoB.data));
        },
      },
      {
        colId: "nota",
        headerName: "Nota",
        flex: 1.4,
        minWidth: 160,
        valueGetter: ({ data }) => (data && data.estado === "con_orden" ? data.orden?.nota || "—" : "—"),
        cellStyle: { textAlign: "left" },
      },
      {
        colId: "acciones",
        headerName: "Acción",
        flex: 0.8,
        minWidth: 110,
        sortable: false,
        filter: false,
        cellRenderer: ({ data }: { data: FilaTelemetria }) => (
          <div className="flex h-full items-center justify-center">
            <button type="button" className="boton boton-celda" onClick={() => cambiar(data)}>
              Cambiar
            </button>
          </div>
        ),
      },
    ],
    [ahora, cambiar],
  );

  const selectorVista = (
    <SegmentadoOpciones opciones={OPCIONES_VISTA} valor={vista} onCambiar={setVista} etiqueta="Vista" />
  );
  const campoBusqueda = (
    <div className="campo flex-[0_1_16rem]">
      <input
        placeholder={vista === "equipos" ? "Equipo, unidad, estudio…" : "Equipo, estudio, quién, nota…"}
        value={busqueda}
        onChange={(evento) => setBusqueda(evento.target.value)}
      />
    </div>
  );
  const cambios = historial.datos ?? [];

  return (
    <div className="flex h-full flex-col">
      <div className="pantalla-cuerpo min-h-0 flex-1">
        <div className="min-h-0 flex-1">
          {/* Cada vista con su `key`: sin ella React reusa la misma grilla y por
              un instante pinta las filas de una con las columnas de la otra. */}
          {vista === "equipos" ? (
            <Tabla<FilaTelemetria>
              key="equipos"
              id="telemetria"
              columnas={columnas}
              filas={filasVisibles}
              busqueda={busqueda}
              filtrosPorColumna
              idFila={(fila) => fila.equipo.id}
              onFilaDobleClic={cambiar}
              controles={
                <>
                  {selectorVista}
                  {campoBusqueda}
                  <SegmentadoOpciones
                    opciones={opcionesFiltro(conteo)}
                    valor={filtro}
                    onCambiar={setFiltro}
                    etiqueta="Filtrar por estado"
                  />
                </>
              }
              accionesDerecha={
                <span
                  className="text-sm text-(--muted)"
                  title="Los errores se reportan siempre y no se controlan desde acá"
                >
                  Sin orden: {textoGrupos(GRUPOS_POR_DEFECTO)}
                </span>
              }
            />
          ) : (
            <Tabla<CambioOrdenTelemetria>
              key="cambios"
              id="telemetria-cambios"
              columnas={COLUMNAS_CAMBIOS}
              filas={cambios}
              busqueda={busqueda}
              filtrosPorColumna
              idFila={(cambio) => String(cambio.id)}
              controles={
                <>
                  {selectorVista}
                  {campoBusqueda}
                  <span role="status">{cambios.length.toLocaleString("es-CR")} cambios</span>
                </>
              }
            />
          )}
        </div>
        {(vista === "equipos" ? cargando && filas.length === 0 : historial.cargando && cambios.length === 0) && (
          <p className="text-muted">Cargando…</p>
        )}
      </div>

      {datos && equipoEditado && (
        <EditarOrden
          equipo={equipoEditado}
          orden={datos.ordenes.get(equipoEditado.id)}
          ahora={datos.cargadoEn}
          onCerrar={() => setEditando(null)}
          onGuardada={() => {
            setEditando(null);
            void recargar();
            void historial.recargar();
          }}
        />
      )}
    </div>
  );
}
