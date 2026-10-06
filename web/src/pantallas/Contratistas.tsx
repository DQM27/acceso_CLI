import { useMemo, useState } from "react";
import { toast } from "sonner";
import { Check } from "lucide-react";
import type { CellStyle, ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import InterruptorCelda from "../componentes/InterruptorCelda";
import AvisoTruncado from "../componentes/AvisoTruncado";
import { useLista } from "../componentes/useLista";
import FormularioContratista from "./FormularioContratista";
import { actualizarAccesoContratista, listarContratistas } from "../api/contratistas";
import type { ContratistaConEstado } from "../api/contratistas";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";
import { COLOR_NIVEL, ESTADOS_ACCESO, nivelPraind, textoEstadoPraind } from "./Contratistas.logica";
import { mensajeError } from "../mensajeError";

// Declarados afuera del array de columnas y ya tipados como `CellStyle` --
// dentro del array, mezclar objetos literales con distintas claves
// (`textAlign` acá, `display`/`justifyContent`/`alignItems` en
// "es_personal_ruta") hace que TS infiera un único tipo combinado para
// todos los elementos y rechace la asignación a `ColDef<ContratistaConEstado>[]`.
const ESTILO_IZQUIERDA: CellStyle = { textAlign: "left" };
const ESTILO_CENTRO_FLEX: CellStyle = { display: "flex", justifyContent: "center", alignItems: "center" };

/**
 * Vista, alta, edición y baja de contratistas. "+ Nuevo" y "Editar" abren
 * `FormularioContratista` (reglas del núcleo vía WebAssembly; guardan las Edge
 * Functions `admin-crear-contratista` / `admin-editar-contratista`). El
 * toggle "Activo" ES la baja (y la reactivación) -- global, no por sitio, ver
 * `api/contratistas.ts`.
 */
export default function Contratistas() {
  const [busqueda, setBusqueda] = useState("");
  const [modalAbierto, setModalAbierto] = useState(false);
  // Contratista que se está editando (modal de edición abierto), o null.
  const [editando, setEditando] = useState<ContratistaConEstado | null>(null);

  // Cambia rara vez (altas/bajas puntuales) -- mismo intervalo que usan
  // desktop/mobile para su propio sync periódico.
  const { datos, cargando, recargar } = useLista(["contratistas"], listarContratistas, {
    intervaloMs: 120_000,
    // `ingresos`: la columna "Adentro" cambia con cada entrada y salida.
    tablas: "contratistas,empresas,ingresos",
  });
  const filas = datos?.filas ?? [];
  const truncado = datos?.truncado ?? false;

  async function manejarEdicion(fila: ContratistaConEstado) {
    try {
      await actualizarAccesoContratista(fila.id, fila.activo);
      toast.success(
        fila.activo ? `${fila.nombre} reactivado.` : `${fila.nombre} dado de baja.`,
      );
    } catch (error) {
      // La grilla ya muestra el valor nuevo (edición optimista de AG Grid) --
      // si el guardado falla, hay que volver a pedir los datos reales para
      // que la celda no quede mintiendo.
      toast.error(mensajeError(error));
      void recargar();
    }
  }

  const columnas = useMemo<ColDef<ContratistaConEstado>[]>(
    () => [
      {
        // Primera columna: a mano también en pantallas angostas, donde la
        // grilla virtualiza las columnas del final.
        colId: "editar",
        headerName: "",
        flex: 0.7,
        minWidth: 90,
        filter: false,
        sortable: false,
        cellRenderer: ({ data }: { data: ContratistaConEstado }) => (
          <button type="button" className="boton boton-celda-angosto" onClick={() => setEditando(data)}>
            Editar
          </button>
        ),
      },
      {
        field: "identificacion",
        // "Cédula", no "Identificación" -- mismo término que usan
        // Historial y Usuarios para el mismo dato (columna `cedula`/
        // `contratista_cedula` ahí, `identificacion` acá por herencia del
        // nombre de columna en Supabase) -- antes cada pantalla le decía
        // distinto a lo mismo.
        headerName: "Cédula",
        flex: 1.3,
        minWidth: 140,
        cellStyle: ESTILO_IZQUIERDA,
      },
      { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: ESTILO_IZQUIERDA },
      { field: "empresa_nombre", headerName: "Empresa", flex: 1.3, minWidth: 140 },
      { field: "tipo_ingreso", headerName: "Tipo", flex: 1.1, minWidth: 110 },
      {
        field: "fecha_vencimiento_praind",
        headerName: "PRAIND vence",
        flex: 1.3,
        minWidth: 130,
        valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
      },
      {
        // Estado de la PRAIND y de las reglas de acceso: los calcula la vista
        // `panel_contratistas_estado` con las mismas reglas que el núcleo
        // (src/domain/acceso.rs), así el panel dice lo mismo que el puesto
        // de control al intentar registrar el ingreso.
        colId: "estado_praind",
        headerName: "PRAIND",
        flex: 1.3,
        minWidth: 140,
        valueGetter: (p) => (p.data ? textoEstadoPraind(p.data) : ""),
        cellStyle: (p) => {
          const nivel = p.data ? nivelPraind(p.data.estado_praind) : null;
          return nivel ? { color: COLOR_NIVEL[nivel], fontWeight: 600 } : null;
        },
      },
      {
        colId: "estado_acceso",
        headerName: "Estado",
        flex: 1.6,
        minWidth: 170,
        valueGetter: (p) => (p.data ? ESTADOS_ACCESO[p.data.estado_acceso].texto : ""),
        cellStyle: (p) =>
          p.data ? { color: COLOR_NIVEL[ESTADOS_ACCESO[p.data.estado_acceso].nivel], fontWeight: 600 } : null,
      },
      {
        // Unidad y desde cuándo, si tiene un ingreso sin salida.
        colId: "adentro",
        headerName: "Adentro",
        flex: 1.5,
        minWidth: 160,
        valueGetter: (p) =>
          p.data?.adentro_desde
            ? `${p.data.adentro_sitio_nombre ?? "—"} · ${textoFechaDDMMYYYY(fechaLocalYMD(p.data.adentro_desde))} ${textoHora(p.data.adentro_desde)}`
            : "",
      },
      {
        // Sólo lectura en la grilla: se cambia con "Editar".
        field: "es_personal_ruta",
        headerName: "Personal de ruta",
        flex: 1.6,
        minWidth: 160,
        valueFormatter: (p) => (p.value ? "Sí" : "No"),
        // Render propio (no InterruptorCelda -- ese es el switch editable
        // que usa desktop/src/pantallas/Contratistas.tsx; acá se cambia con
        // "Editar") -- así el
        // ícono queda centrado y en verde cuando es "Sí" en vez del check
        // gris por defecto que AG Grid le pone a un campo booleano.
        cellStyle: ESTILO_CENTRO_FLEX,
        cellRenderer: ({ value }: { value: boolean }) =>
          value ? <Check size={16} color="var(--exito)" strokeWidth={2.5} aria-label="Sí" /> : null,
        filter: false,
      },
      {
        field: "activo",
        headerName: "Activo",
        flex: 0.9,
        minWidth: 100,
        cellRenderer: InterruptorCelda,
        cellRendererParams: { critico: true },
        filter: false,
      },
    ],
    [],
  );

  return (
    <div className="flex h-full flex-col">
      <div className="pantalla-cuerpo min-h-0 flex-1">
        {truncado && (
          <AvisoTruncado
            mensaje={`Hay más de ${filas.length.toLocaleString("es-CR")} contratistas -- se muestran solo los primeros (la búsqueda de aquí arriba sólo filtra entre esos, no trae más).`}
          />
        )}
        <div className="min-h-0 flex-1">
          <Tabla<ContratistaConEstado>
            id="contratistas"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            filtrosPorColumna
            onCeldaEditada={manejarEdicion}
            controles={
              <>
                <button type="button" className="boton" onClick={() => setModalAbierto(true)}>
                  + Nuevo
                </button>
                <div className="campo flex-[0_1_16rem]">
                  <input
                    placeholder="Cédula o nombre…"
                    value={busqueda}
                    disabled={cargando}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
              </>
            }
          />
        </div>
      </div>

      {editando && (
        <FormularioContratista
          contratista={editando}
          onGuardado={() => {
            setEditando(null);
            void recargar();
          }}
          onCerrar={() => setEditando(null)}
        />
      )}

      {modalAbierto && (
        <FormularioContratista
          onGuardado={() => {
            setModalAbierto(false);
            void recargar();
          }}
          onCerrar={() => setModalAbierto(false)}
        />
      )}
    </div>
  );
}
