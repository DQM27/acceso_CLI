import { Suspense, lazy, useCallback, useEffect, useMemo, useRef, useState } from "react";
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
import { ESTILO_BOTON_COMPACTO } from "../componentes/MenuAcciones.logica";
import { cerrarActivo, listarActivosUnificados } from "../api/activosUnificados";
import type { ActivoUnificado, ClaseActivo } from "../api/activosUnificados";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import {
  contarPorClase,
  contarPorGafete,
  ETIQUETA_CLASE,
  filtrarPorClase,
  filtrarPorGafete,
  filtrarPorNivel,
} from "./Activos.logica";
import type { FiltroClase, FiltroGafete, FiltroPermanencia } from "./Activos.logica";

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

const IngresoProveedorModal = lazy(() => import("./IngresoProveedorModal"));
const EntregarGafeteProvisionalModal = lazy(() => import("./EntregarGafeteProvisionalModal"));
const NuevaVisitaModal = lazy(() => import("./NuevaVisitaModal"));

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
function claseFilaActiva(fila: ActivoUnificado): string | undefined {
  if (fila.nivel_permanencia === "Critico") return "fila-permanencia-critico";
  if (fila.nivel_permanencia === "Aviso") return "fila-permanencia-aviso";
  return undefined;
}

export default function Activos({
  refrescarSenal,
  onAbrirNuevoIngreso,
  onAbrirSalida,
  filtroPermanencia,
}: {
  /** Los modales de Nuevo Ingreso y Salida viven en el Shell (se disparan
   * desde cualquier pantalla vía Ctrl+N/S, no sólo desde acá) —
   * este número sube cada vez que registran algo, para que la grilla se
   * refresque aunque ya estuviera montada. */
  refrescarSenal?: number;
  onAbrirNuevoIngreso: () => void;
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
  const [modalProveedor, setModalProveedor] = useState(false);
  const [modalKof, setModalKof] = useState(false);
  const [modalCorreo, setModalCorreo] = useState(false);
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
  // Los números de los botones de los filtros (ver `opcionesFiltroGafete` y
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

  const recargar = useCallback(() => {
    // `Promise.resolve().then(...)` en vez de llamar `setCargando(true)`
    // directo -- de lo contrario `react-hooks/set-state-in-effect` marca
    // esta actualización de estado como síncrona dentro del cuerpo del
    // efecto que la dispara (abajo). Diferirla a un microtask no cambia
    // nada perceptible (corre antes del próximo paint igual) y cumple la
    // regla.
    return Promise.resolve()
      .then(() => setCargando(true))
      .then(() => listarActivosUnificados())
      .then(({ activos, total }) => {
        setActivos(activos);
        setTotal(total);
        setCargadoAlgunaVez(true);
        setSeleccionadas([]);
      })
      .finally(() => setCargando(false));
  }, []);

  useEffect(() => {
    let vigente = true;
    recargar().catch((error) => vigente && avisos.error(error));
    return () => {
      vigente = false;
    };
  }, [recargar, refrescarSenal]);

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
      for (const fila of seleccionadas) {
        await cerrarActivo(fila);
      }
      setConfirmarSalidaMasiva(false);
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
  const columnas: ColDef<ActivoUnificado>[] = useMemo(
    () => [
      {
        field: "cedula",
        headerName: "Cédula",
        flex: 1.2,
        minWidth: 120,
        cellStyle: { textAlign: "left" },
        // El préstamo KOF es de un encargado con código de empleado, sin cédula.
        valueFormatter: (p) => p.value ?? "—",
      },
      {
        field: "nombre",
        headerName: "Nombre",
        flex: 1.6,
        minWidth: 170,
        cellStyle: { textAlign: "left" },
      },
      // Empresa (contratista, proveedor), motivo (por correo) o código de
      // empleado (KOF): lo que acompaña al nombre.
      { field: "detalle", headerName: "Empresa / motivo", flex: 1.3, minWidth: 150 },
      {
        field: "tipo",
        headerName: "Tipo",
        flex: 1,
        minWidth: 100,
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
        minWidth: 100,
        getQuickFilterText: () => "",
      },
      {
        field: "gafete_numero",
        type: "numero",
        headerName: "Gafete",
        flex: 0.9,
        minWidth: 90,
        valueFormatter: (p) => (p.value == null ? "S/G" : String(p.value)),
        getQuickFilterText: () => "",
      },
      {
        colId: "fecha_ingreso",
        type: "fecha",
        headerName: "Fecha",
        flex: 1.1,
        minWidth: 110,
        // `valueGetter` (no `field`) a propósito, igual que Hora: si la
        // columna lee el string ISO crudo, AG Grid la infiere como fecha y
        // el filtro flotante termina siendo el selector nativo de
        // fecha+hora del navegador (`datetime-local`) en vez de un texto
        // simple — igual que le pasaba a Hora. `fechaLocalYMD` mantiene el
        // orden cronológico correcto como texto ("2026-08-28" ordena bien
        // sin volver a pasar por `Date`); `valueFormatter` sólo reacomoda
        // ese mismo string a DD/MM/AAAA para mostrar, sin re-parsearlo.
        valueGetter: (p) => (p.data ? fechaLocalYMD(p.data.fecha_hora_ingreso) : ""),
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
        getQuickFilterText: () => "",
      },
      {
        colId: "hora_ingreso",
        headerName: "Hora",
        flex: 0.9,
        minWidth: 90,
        // `valueGetter` (no `field`+`valueFormatter`) a propósito: si lee
        // el string ISO crudo, AG Grid vuelve a inferirla como fecha (con
        // el mismo problema de la columna Fecha). Devolviendo ya el texto
        // "HH:MM", la columna se filtra/ordena como texto plano — mismo
        // comportamiento que el resto de columnas de texto, sin ícono raro.
        valueGetter: (p) => (p.data ? textoHora(p.data.fecha_hora_ingreso) : ""),
        getQuickFilterText: () => "",
      },
      {
        field: "usuario_ingreso_nombre",
        headerName: "Dio ingreso",
        flex: 1.3,
        minWidth: 130,
        getQuickFilterText: () => "",
      },
      {
        headerName: "Acción",
        flex: 0.9,
        minWidth: 90,
        filter: false,
        sortable: false,
        // Sin pinned a propósito: una columna fijada reserva el ancho del
        // scrollbar vertical aunque no haga falta scroll, dejando un
        // espacio en blanco justo antes de ella — con las 10 columnas ya
        // entrando sin scroll horizontal (ver el ajuste de anchos previo),
        // fijarla no aporta nada y sí ese espacio no deseado.
        cellRenderer: (p: ICellRendererParams<ActivoUnificado>) => {
          const fila = p.data;
          return fila ? (
            <button
              type="button"
              className="boton"
              style={{ padding: "0.15rem 0.55rem", fontSize: "0.78rem" }}
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

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
          <Tabla<ActivoUnificado>
            ref={tablaRef}
            cargando={cargando}
            filtrosPorColumna
            id="activos"
            idFila={(fila) => fila.clave}
            claseFila={claseFilaActiva}
            columnas={columnas}
            filas={filasVisibles}
            busqueda={busqueda}
            seleccionMultiple
            onSeleccionCambia={setSeleccionadas}
            accionesDerecha={
              <div className="campo" style={{ flex: "0 0 20rem" }}>
                <input
                  placeholder="Cédula, nombre, empresa o motivo…"
                  value={busqueda}
                  onChange={(evento) => setBusqueda(evento.target.value)}
                />
              </div>
            }
            controles={
              <>
                {/* Tres bloques de izquierda a derecha: qué registrar, qué
                    ver (clase y gafete juntos) y, al otro extremo, el
                    buscador. "Ingreso" abre la lista de qué registrar; el
                    atajo de cada uno se ve en la lista (Ctrl+N) y al lado
                    de "Salida" (Ctrl+S). */}
                <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
                  <MenuAcciones
                    texto="Ingreso"
                    Icono={LogIn}
                    acciones={[
                      { texto: "Contratista", Icono: HardHat, atajo: "Ctrl+N", onElegir: onAbrirNuevoIngreso },
                      { texto: "Proveedor", Icono: Boxes, onElegir: () => setModalProveedor(true) },
                      { texto: "Visita por correo", Icono: Mail, onElegir: () => setModalCorreo(true) },
                      { texto: "Gafete KOF", Icono: BadgeCheck, onElegir: () => setModalKof(true) },
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
                    <span style={{ opacity: 0.6, fontSize: "0.72rem" }}>Ctrl+S</span>
                  </button>
                </div>
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

      {/* Proveedor y KOF se registran desde acá (menú "Nuevo ingreso"); sus
          modales ya existían, antes colgaban de pantallas propias. */}
      <Suspense fallback={null}>
        {modalProveedor && (
          <IngresoProveedorModal
            onRegistrado={() => {
              setModalProveedor(false);
              recargar();
            }}
            onCerrar={() => setModalProveedor(false)}
          />
        )}
        {modalCorreo && (
          <NuevaVisitaModal
            modoInicial="correo"
            onRegistrado={recargar}
            onCerrar={() => setModalCorreo(false)}
          />
        )}
        {modalKof && (
          <EntregarGafeteProvisionalModal
            onRegistrado={() => {
              setModalKof(false);
              recargar();
            }}
            onCerrar={() => setModalKof(false)}
          />
        )}
      </Suspense>

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
              {procesando ? "Registrando…" : "Confirmar"}
            </button>
          </div>
        </Modal>
      )}
    </div>
  );
}
