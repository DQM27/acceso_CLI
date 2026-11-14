import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { avisos } from "../avisos/avisos";
import { BadgeCheck, Boxes, CircleOff, HardHat, IdCard, Layers, LogIn, LogOut, Mail, Users } from "lucide-react";
import type { LucideIcon } from "lucide-react";
import type { ColDef, ICellRendererParams } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import type { TablaHandle } from "../componentes/Tabla";
import BarraSeleccion from "../componentes/BarraSeleccion";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import Modal from "../componentes/Modal";
import MenuAcciones from "../componentes/MenuAcciones";
import { useRecargaVisible } from "../componentes/useRecargaVisible";
import { idPorClave } from "../componentes/Tabla.logica";
import { ESTILO_BOTON_COMPACTO } from "../componentes/MenuAcciones.logica";
import { cerrarActivo, cerrarActivos, listarActivosUnificados } from "../api/activosUnificados";
import type { ActivoUnificado, ClaseActivo } from "../api/activosUnificados";
import { columnaGafete } from "../componentes/columnasComunes";
import { textoDuracion, textoIngresoCorto } from "../tiempo";
import {
  CLASE_SINGULAR,
  contarPorClase,
  contarPorGafete,
  ETIQUETA_CLASE,
  filtrarPorClase,
  filtrarPorGafete,
  filtrarPorNivel,
  textoRechazos,
} from "./Activos.logica";
import type { FiltroClase, FiltroGafete, FiltroPermanencia, OtroIngreso } from "./Activos.logica";

/** Ícono, número y nombre completo (al pasar el mouse) de cada opción. Los
 * números son el conteo de gente adentro: el total, y cuántos tienen gafete y
 * cuántos no (S/G). Mientras carga no se muestran, para no enseñar ceros
 * falsos. */
function opcionesFiltroGafete(
  conteo: { todos: number; con: number; sin: number } | null,
): OpcionSegmentada<FiltroGafete>[] {
  return [
    {
      valor: "todos",
      Icono: Users,
      titulo: conteo ? `${conteo.todos} adentro` : "Todos",
      cantidad: conteo?.todos,
    },
    {
      valor: "con",
      Icono: IdCard,
      titulo: conteo ? `${conteo.con} con gafete` : "Con gafete",
      cantidad: conteo?.con,
    },
    {
      valor: "sin",
      Icono: CircleOff,
      // Sólo los contratistas pueden entrar sin gafete: proveedores, visitas por
      // correo y KOF lo llevan siempre.
      titulo: conteo
        ? `${conteo.sin} sin gafete (S/G) — sólo los contratistas pueden entrar sin gafete`
        : "Sin gafete (S/G) — sólo los contratistas pueden entrar sin gafete",
      cantidad: conteo?.sin,
    },
  ];
}

/** Pieza única de íconos con el relleno deslizante (`SegmentadoOpciones`). */
function ToggleGafete({
  filtro,
  conteo,
  onCambiar,
}: {
  filtro: FiltroGafete;
  conteo: { todos: number; con: number; sin: number } | null;
  onCambiar: (filtro: FiltroGafete) => void;
}) {
  return (
    <SegmentadoOpciones
      opciones={opcionesFiltroGafete(conteo)}
      valor={filtro}
      onCambiar={onCambiar}
      etiqueta="Filtrar por gafete"
    />
  );
}

const ICONO_CLASE: Record<ClaseActivo, LucideIcon> = {
  contratista: HardHat,
  proveedor: Boxes,
  correo: Mail,
  kof: BadgeCheck,
};

const CLASES: ClaseActivo[] = ["contratista", "proveedor", "correo", "kof"];

/** Una opción por clase más "Todas", con la cantidad de cada una. Mientras
 * carga no hay números, igual que el filtro de gafete. */
function opcionesFiltroClase(
  conteo: Record<ClaseActivo, number> | null,
): OpcionSegmentada<FiltroClase>[] {
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


/** El resaltado de la fila sale del nivel que calculó el núcleo (sólo los
 * contratistas lo tienen). */
const REGLAS_PERMANENCIA: Record<string, (fila: ActivoUnificado) => boolean> = {
  "fila-permanencia-critico": (fila) => fila.nivel_permanencia === "Critico",
  "fila-permanencia-aviso": (fila) => fila.nivel_permanencia === "Aviso",
};

export default function Activos({
  refrescarSenal,
  onAbrirNuevoIngreso,
  onAbrirOtroIngreso,
  onAbrirSalida,
  filtroPermanencia,
}: {
  /** Los modales de ingreso (contratista, proveedor, visita por correo y KOF)
   * y el de Salida viven en el Shell (se disparan desde cualquier pantalla con
   * su atajo, no sólo desde acá) — este número sube cada vez que registran algo,
   * para que la grilla se refresque aunque ya estuviera montada. */
  refrescarSenal?: number;
  onAbrirNuevoIngreso: () => void;
  onAbrirOtroIngreso: (clase: OtroIngreso) => void;
  onAbrirSalida: () => void;
  /** Si hay un aviso de permanencia pulsado en la barra de estado, la tabla
   * se limita a esas personas (ver `Shell`). */
  filtroPermanencia: FiltroPermanencia | null;
}) {
  const [activos, setActivos] = useState<ActivoUnificado[]>([]);
  const [total, setTotal] = useState(0);
  const [cargando, setCargando] = useState(true);
  // Los números del filtro sólo se ocultan en la PRIMERA carga: en las
  // siguientes (cada ingreso o salida recarga) se quedan los anteriores hasta
  // que llegan los nuevos, en vez de parpadear.
  const [cargadoAlgunaVez, setCargadoAlgunaVez] = useState(false);
  const [busqueda, setBusqueda] = useState("");
  const [seleccionadas, setSeleccionadas] = useState<ActivoUnificado[]>([]);
  const [confirmarSalidaMasiva, setConfirmarSalidaMasiva] = useState(false);
  const [procesando, setProcesando] = useState(false);
  const [filtroGafete, setFiltroGafete] = useState<FiltroGafete>("todos");
  const [filtroClase, setFiltroClase] = useState<FiltroClase>("todas");
  // Primero la clase, después el gafete y la permanencia: los botones de gafete
  // cuentan dentro de la clase elegida.
  const activosDeClase = useMemo(
    () => filtrarPorClase(activos, filtroClase),
    [activos, filtroClase],
  );
  // El filtro de gafete es sólo de contratistas, los únicos que pueden entrar
  // sin gafete: con "Todas" se ve todo sin filtrar, y en las demás clases
  // siempre hay uno ("sin gafete" sería 0 y confunde). Al esconderse se
  // ignora, para que un "sin gafete" elegido antes no deje la tabla vacía sin
  // un botón que lo apague.
  const mostrarFiltroGafete = filtroClase === "contratista";
  const filtroGafeteEfectivo = mostrarFiltroGafete ? filtroGafete : "todos";
  const filasVisibles = useMemo(
    () =>
      filtrarPorNivel(filtrarPorGafete(activosDeClase, filtroGafeteEfectivo), filtroPermanencia),
    [activosDeClase, filtroGafeteEfectivo, filtroPermanencia],
  );
  // Los números de los filtros (ver `opcionesFiltroGafete` y
  // `opcionesFiltroClase`): son la única métrica de la pantalla, ya no hay texto
  // en la barra de estado. `total` sólo vale para "todas": es el de contratistas
  // más las otras clases.
  const conteo = useMemo(
    () =>
      cargadoAlgunaVez
        ? {
            todos: filtroClase === "todas" ? total : activosDeClase.length,
            ...contarPorGafete(activosDeClase),
          }
        : null,
    [cargadoAlgunaVez, total, filtroClase, activosDeClase],
  );
  const conteoClases = useMemo(
    () => (cargadoAlgunaVez ? contarPorClase(activos) : null),
    [cargadoAlgunaVez, activos],
  );

  // La acción para las filas seleccionadas flota sobre la tabla (`BarraSeleccion`):
  // ni corre la pantalla -- antes una franja arriba lo hacía (pedido del usuario
  // 2026-09-23) -- ni queda escondida en la barra de estado.
  const tablaRef = useRef<TablaHandle<ActivoUnificado>>(null);

  // La selección NO se borra al recargar: cada sincronización o aviso en vivo
  // recarga, y antes desmarcaba las casillas a mitad de una salida masiva. AG
  // Grid ya saca de la selección las filas que desaparecen (y avisa por
  // `onSeleccionCambia`), así que sólo quedan marcadas las que siguen adentro.
  // `cargando` sólo vale para la PRIMERA carga (la tabla no muestra "Cargando…"
  // después): arranca en `true` y se apaga al terminar, no se vuelve a prender
  // en cada recarga -- así tampoco hace falta un setState síncrono dentro del
  // efecto (antes se diferían con `Promise.resolve().then` para la regla
  // `react-hooks/set-state-in-effect`).
  const recargar = useCallback(() => {
    return listarActivosUnificados()
      .then(({ activos, total }) => {
        setActivos(activos);
        setTotal(total);
        setCargadoAlgunaVez(true);
      })
      .finally(() => setCargando(false));
  }, []);

  // Oculta (otra sección a la vista), la pantalla no recarga: queda
  // pendiente y recarga una vez al volver (ver `useRecargaVisible`).
  const versionDatos = useRecargaVisible(refrescarSenal);
  useEffect(() => {
    let vigente = true;
    recargar().catch((error) => vigente && avisos.error(error));
    return () => {
      vigente = false;
    };
  }, [recargar, versionDatos]);

  // useCallback a propósito: esta función se cierra dentro de una celda de
  // `columnas` — sin identidad estable, `columnas` (memoizado más abajo)
  // se recrearía en cada render igual, y con eso AG Grid reasignaría el
  // orden/ancho originales del código encima de lo que el usuario acomodó
  // (ver el comentario junto a `columnas`).
  const salidaIndividual = useCallback(
    async (fila: ActivoUnificado) => {
      try {
        await cerrarActivo(fila);
        recargar();
      } catch (error) {
        avisos.error(error);
      }
    },
    [recargar],
  );

  async function confirmarSalidasSeleccionadas() {
    setProcesando(true);
    try {
      // Un solo pedido al núcleo para todos los contratistas (locales y del
      // otro dispositivo) y un solo aviso de sincronización; lo que no se
      // pudo cerrar vuelve con su motivo y no frena al resto.
      const { rechazados } = await cerrarActivos(seleccionadas);
      setConfirmarSalidaMasiva(false);
      if (rechazados.length > 0) avisos.error(textoRechazos(rechazados, seleccionadas.length));
      await recargar();
    } catch (error) {
      avisos.error(error);
    } finally {
      setProcesando(false);
    }
  }

  // useMemo a propósito: si `columnas` se recrea en cada render (ej. cada
  // vez que `recargar()` trae datos nuevos), AG Grid recibe un `columnDefs`
  // "nuevo" y reaplica el orden/ancho literales de acá encima de lo que el
  // usuario ya había acomodado a mano — el layout persistido en
  // `Tabla`/localStorage quedaba pisado en el siguiente refresco.
  // Columnas rediseñadas el 2026-11-14 (auditoría, U1/U2): "Acción" fijada a
  // la derecha (antes quedaba cortada a 1440 px o menos), "Clase" sólo con
  // "Todas" (con un filtro puesto repetía lo mismo en cada fila), Fecha y
  // Hora juntas en "Ingreso", "Tiempo adentro" (lo que de verdad se vigila,
  // antes había que restar a mano) y "Dio ingreso" como columna opcional.
  // "Clase" sólo con "Todas": se oculta sin sacarla del arreglo, así vuelve a
  // su lugar (ver `columnasOcultasPorPantalla` de Tabla).
  const ocultasPorClase = useMemo(() => (filtroClase === "todas" ? [] : ["clase"]), [filtroClase]);
  const columnas: ColDef<ActivoUnificado>[] = useMemo(
    () => [
      // A qué grupo pertenece la fila de un vistazo (la columna "Tipo" de más
      // abajo es otra cosa: PRAIND/IN_HOUSE de un contratista, etc.).
      {
        colId: "clase",
        headerName: "Clase",
        flex: 1.1,
        minWidth: 110,
        valueGetter: (p) => (p.data ? CLASE_SINGULAR[p.data.clase] : ""),
        getQuickFilterText: () => "",
      },
      {
        field: "cedula",
        headerName: "Cédula",
        flex: 1.1,
        minWidth: 105,
        cellStyle: { textAlign: "left" },
        // El préstamo KOF es de un encargado con código de empleado, sin cédula.
        valueFormatter: (p) => p.value ?? "—",
      },
      {
        field: "nombre",
        headerName: "Nombre",
        flex: 2,
        minWidth: 165,
        cellStyle: { textAlign: "left" },
      },
      // Empresa (contratista, proveedor), motivo (por correo) o código de
      // empleado (KOF): lo que acompaña al nombre.
      { field: "detalle", headerName: "Empresa / motivo", flex: 1.4, minWidth: 125 },
      {
        field: "tipo",
        headerName: "Tipo",
        flex: 0.9,
        minWidth: 90,
        // El texto ya viene armado en la fila (`ActivoUnificado.tipo`): el filtro
        // por columna, el orden y las exportaciones usan el mismo que se ve.
        // Buscador de arriba (quickFilter) limitado a Cédula/Nombre/Empresa
        // -- las tres cosas que identifican a LA PERSONA que se busca.
        // `getQuickFilterText: () => ""` saca esta columna de esa búsqueda
        // sin afectar el filtro de columna propio (`floatingFilter`), que
        // sigue funcionando normal. Mismo criterio en el resto de columnas
        // de abajo -- hallazgo real del usuario 2026-09-21: buscar "daniel"
        // (un contratista) traía en cambio el registro dado de alta por el
        // operador Daniel, porque el buscador también miraba "Dio ingreso".
        getQuickFilterText: () => "",
      },
      {
        field: "medio",
        headerName: "Medio",
        flex: 1,
        minWidth: 115,
        getQuickFilterText: () => "",
      },
      columnaGafete({ getQuickFilterText: () => "" }),
      {
        // El ISO crudo ordena cronológicamente; se ve la hora, y la fecha
        // sólo si no fue hoy (`textoIngresoCorto`).
        colId: "ingreso",
        headerName: "Ingreso",
        flex: 0.9,
        minWidth: 100,
        valueGetter: (p) => p.data?.fecha_hora_ingreso ?? "",
        valueFormatter: (p) => (p.value ? textoIngresoCorto(p.value) : ""),
        filterValueGetter: (p) => (p.data ? textoIngresoCorto(p.data.fecha_hora_ingreso) : ""),
        getQuickFilterText: () => "",
      },
      {
        // Milisegundos desde el ingreso (ordena de menos a más tiempo);
        // se refresca cada minuto sin recargar datos (ver el efecto de abajo).
        colId: "tiempo_adentro",
        // "Tiempo" a secas en el encabezado para que no se corte a 1366 px; el
        // nombre completo queda en el tooltip y en "Tabla" > Columnas.
        headerName: "Tiempo",
        headerTooltip: "Tiempo adentro",
        flex: 1,
        minWidth: 110,
        filter: false,
        enableCellChangeFlash: false,
        valueGetter: (p) => (p.data ? Date.now() - Date.parse(p.data.fecha_hora_ingreso) : null),
        valueFormatter: (p) => (typeof p.value === "number" ? textoDuracion(p.value) : ""),
        getQuickFilterText: () => "",
      },
      {
        field: "usuario_ingreso_nombre",
        headerName: "Dio ingreso",
        flex: 1.2,
        minWidth: 130,
        // Opcional: se activa desde "Tabla" > Columnas visibles.
        hide: true,
        getQuickFilterText: () => "",
      },
      {
        colId: "accion",
        headerName: "Acción",
        // Fijada a la derecha: siempre visible, aunque falte ancho para el
        // resto de las columnas.
        pinned: "right",
        lockPinned: true,
        width: 96,
        minWidth: 96,
        resizable: false,
        filter: false,
        sortable: false,
        cellRenderer: (p: ICellRendererParams<ActivoUnificado>) => {
          const fila = p.data;
          return fila ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "var(--tam-xs)" }}
              onClick={() => salidaIndividual(fila)}
            >
              {/* En KOF no sale una persona: se devuelve el gafete. */}
              Salida
            </button>
          ) : null;
        },
      },
    ],
    [salidaIndividual],
  );

  // "Tiempo adentro" cambia con el reloj: se recalcula cada minuto, sin
  // volver a pedir los datos.
  useEffect(() => {
    const reloj = window.setInterval(() => tablaRef.current?.refrescarColumnas(["tiempo_adentro"]), 60_000);
    return () => window.clearInterval(reloj);
  }, []);

  // Qué decir con la tabla vacía, según la clase elegida (U8).
  const mensajeVacio =
    filtroClase === "todas"
      ? "No hay nadie adentro. Ctrl+N registra el ingreso de un contratista."
      : `No hay ${ETIQUETA_CLASE[filtroClase].toLowerCase()} adentro.`;

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
          <Tabla<ActivoUnificado>
            ref={tablaRef}
            cargando={cargando}
            filtrosPorColumna
            id="activos"
            idFila={idPorClave}
            reglasClaseFila={REGLAS_PERMANENCIA}
            columnas={columnas}
            filas={filasVisibles}
            mensajeVacio={mensajeVacio}
            columnasOcultasPorPantalla={ocultasPorClase}
            busqueda={busqueda}
            seleccionMultiple
            onSeleccionCambia={setSeleccionadas}
            accionesDerecha={
              <div className="campo" style={{ flex: "1 1 18rem", minWidth: "10rem", maxWidth: "20rem" }}>
                <input
                  placeholder="Cédula, nombre, empresa o motivo…"
                  value={busqueda}
                  onChange={(evento) => setBusqueda(evento.target.value)}
                />
              </div>
            }
            controles={
              <>
                {/* Qué registrar y qué ver a la izquierda; el buscador, al
                    otro extremo. "Ingreso" abre la lista de qué registrar con
                    el atajo de cada uno; el de "Salida" (Ctrl+S) está en su
                    título. */}
                <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                  <MenuAcciones
                    texto="Ingreso"
                    Icono={LogIn}
                    acciones={[
                      { texto: "Contratista", Icono: HardHat, atajo: "Ctrl+N", onElegir: onAbrirNuevoIngreso },
                      { texto: "Proveedor", Icono: Boxes, atajo: "Alt+P", onElegir: () => onAbrirOtroIngreso("proveedor") },
                      { texto: "Visita por correo", Icono: Mail, atajo: "Alt+C", onElegir: () => onAbrirOtroIngreso("correo") },
                      { texto: "Provisional KOF", Icono: BadgeCheck, atajo: "Alt+K", onElegir: () => onAbrirOtroIngreso("kof") },
                    ]}
                  />
                  <button
                    type="button"
                    className="boton boton-icono"
                    style={{ ...ESTILO_BOTON_COMPACTO, flexShrink: 0 }}
                    title="Registrar salida (Ctrl+S)"
                    onClick={onAbrirSalida}
                  >
                    <LogOut size={15} aria-hidden="true" />
                    Salida
                  </button>
                </div>
                {/* Clases y gafete con íconos y cantidad: toda la barra entra
                    en una sola línea (decisión del dueño, 2026-11-14); el
                    nombre de cada opción se ve al pasar el mouse. */}
                <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", marginLeft: "0.5rem" }}>
                  <SegmentadoOpciones
                    opciones={opcionesFiltroClase(conteoClases)}
                    valor={filtroClase}
                    onCambiar={setFiltroClase}
                    etiqueta="Filtrar por clase"
                  />
                  {mostrarFiltroGafete && (
                    <ToggleGafete filtro={filtroGafete} conteo={conteo} onCambiar={setFiltroGafete} />
                  )}
                </div>
              </>
            }
          />
          <BarraSeleccion
            cantidad={seleccionadas.length}
            etiqueta={{ singular: "seleccionado", plural: "seleccionados" }}
            accion={
              <>
                <LogOut size={16} aria-hidden="true" />
                Registrar salida
              </>
            }
            critica
            onAccion={() => setConfirmarSalidaMasiva(true)}
            onLimpiar={() => tablaRef.current?.deseleccionarTodo()}
          />
        </div>
      </div>

      {confirmarSalidaMasiva && (
        <Modal titulo="Confirmar salida" onCerrar={() => setConfirmarSalidaMasiva(false)}>
          <p style={{ marginTop: 0 }}>
            ¿Registrar la salida de {seleccionadas.length} registro(s)?
            {seleccionadas.some((fila) => fila.clase === "kof") &&
              " En KOF se registra la devolución del gafete."}
          </p>
          <ul style={{ margin: "0 0 1rem", paddingLeft: "1.2rem", color: "var(--muted)" }}>
            {seleccionadas.map((fila) => (
              <li key={fila.clave}>
                {fila.nombre} — {ETIQUETA_CLASE[fila.clase]}
              </li>
            ))}
          </ul>
          <div style={{ display: "flex", justifyContent: "flex-end", gap: "0.5rem" }}>
            <button
              type="button"
              className="boton"
              onClick={() => setConfirmarSalidaMasiva(false)}
              disabled={procesando}
            >
              Cancelar
            </button>
            <button
              type="button"
              className="boton boton-primario"
              onClick={confirmarSalidasSeleccionadas}
              disabled={procesando}
            >
              {procesando
                ? "Registrando…"
                : seleccionadas.length === 1
                  ? "Registrar 1 salida"
                  : `Registrar ${seleccionadas.length} salidas`}
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
