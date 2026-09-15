import { useCallback, useMemo, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { toast } from "sonner";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import InterruptorCelda from "../componentes/InterruptorCelda";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import FormularioVehiculoRuta from "./FormularioVehiculoRuta";
import FormularioEncargadoRuta from "./FormularioEncargadoRuta";
import {
  actualizarEncargadoRuta,
  actualizarVehiculoRuta,
  listarEncargadosRuta,
  listarVehiculosRuta,
} from "../api";
import type { EncargadoRuta, VehiculoRuta } from "../api";

type Vista = "vehiculos" | "encargados";

const ETIQUETAS_VISTA: Record<Vista, string> = {
  vehiculos: "Vehículos",
  encargados: "Encargados",
};

function ToggleVista({ vista, onCambiar }: { vista: Vista; onCambiar: (v: Vista) => void }) {
  return (
    <div style={{ display: "flex", gap: "0.25rem" }}>
      {(Object.keys(ETIQUETAS_VISTA) as Vista[]).map((opcion) => (
        <button
          key={opcion}
          type="button"
          className={opcion === vista ? "boton boton-primario" : "boton"}
          disabled={opcion === vista}
          onClick={() => onCambiar(opcion)}
        >
          {ETIQUETAS_VISTA[opcion]}
        </button>
      ))}
    </div>
  );
}

/**
 * Catálogo de vehículos y encargados KOF (`docs/planes-implementados/plan-control-rutas.md`)
 * -- mismo armazón que Empresas.tsx (grid + interruptor de "Activo" en la
 * propia celda, formulario en modal para el resto de campos), con el
 * toggle "Vehículos/Encargados" de Visitas.tsx para compartir una sola
 * pantalla entre los dos catálogos en vez de dos secciones separadas en el
 * sidebar.
 */
export default function CatalogoRutas() {
  const [vista, setVista] = useState<Vista>("vehiculos");
  const [busqueda, setBusqueda] = useState("");
  const [vehiculos, setVehiculos] = useState<VehiculoRuta[]>([]);
  const [encargados, setEncargados] = useState<EncargadoRuta[]>([]);
  const [cargando, setCargando] = useState(true);
  const [formularioVehiculo, setFormularioVehiculo] = useState<"crear" | VehiculoRuta | null>(
    null,
  );
  const [formularioEncargado, setFormularioEncargado] = useState<"crear" | EncargadoRuta | null>(
    null,
  );

  const total = vista === "vehiculos" ? vehiculos.length : encargados.length;
  useBarraEstado(cargando ? "Cargando…" : `${total} resultado(s)`);

  useHotkeys(
    "ctrl+n",
    () => (vista === "vehiculos" ? setFormularioVehiculo("crear") : setFormularioEncargado("crear")),
    { preventDefault: true },
  );

  const recargar = useCallback(
    (estaVigente: () => boolean = () => true) => {
      setCargando(true);
      const carga = vista === "vehiculos" ? listarVehiculosRuta() : listarEncargadosRuta();
      return carga
        .then((datos) => {
          if (!estaVigente()) return;
          if (vista === "vehiculos") setVehiculos(datos as VehiculoRuta[]);
          else setEncargados(datos as EncargadoRuta[]);
        })
        .finally(() => {
          if (estaVigente()) setCargando(false);
        });
    },
    [vista],
  );

  useCargaAlCambiar(recargar, true);

  async function manejarEdicionVehiculo(fila: VehiculoRuta) {
    try {
      await actualizarVehiculoRuta(fila.id, {
        numero_unidad: fila.numero_unidad,
        placa: fila.placa,
        activo: fila.activo,
      });
    } catch (error) {
      toast.error(String(error));
      recargar();
    }
  }

  async function manejarEdicionEncargado(fila: EncargadoRuta) {
    try {
      await actualizarEncargadoRuta(fila.id, {
        codigo_empleado: fila.codigo_empleado,
        nombre: fila.nombre,
        activo: fila.activo,
      });
    } catch (error) {
      toast.error(String(error));
      recargar();
    }
  }

  const columnasVehiculos: ColDef<VehiculoRuta>[] = useMemo(
    () => [
      { field: "placa", headerName: "Placa", flex: 1.2, minWidth: 120, cellStyle: { textAlign: "left" } },
      {
        field: "numero_unidad",
        headerName: "N.° de unidad",
        flex: 1,
        minWidth: 120,
        valueFormatter: (p) => p.value ?? "—",
      },
      {
        field: "activo",
        headerName: "Activo",
        flex: 0.8,
        minWidth: 100,
        cellRenderer: InterruptorCelda,
        cellRendererParams: { critico: true },
      },
    ],
    [],
  );

  const columnasEncargados: ColDef<EncargadoRuta>[] = useMemo(
    () => [
      {
        field: "codigo_empleado",
        headerName: "Código de empleado",
        flex: 1,
        minWidth: 140,
        cellStyle: { textAlign: "left" },
      },
      { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
      {
        field: "activo",
        headerName: "Activo",
        flex: 0.8,
        minWidth: 100,
        cellRenderer: InterruptorCelda,
        cellRendererParams: { critico: true },
      },
    ],
    [],
  );

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          {vista === "vehiculos" ? (
            <Tabla<VehiculoRuta>
              id="rutas-vehiculos"
              columnas={columnasVehiculos}
              filas={vehiculos}
              busqueda={busqueda}
              onCeldaEditada={manejarEdicionVehiculo}
              onFilaDobleClic={setFormularioVehiculo}
              controles={
                <>
                  <button
                    className="boton"
                    title="Ctrl+N"
                    onClick={() => setFormularioVehiculo("crear")}
                  >
                    + Nuevo
                  </button>
                  <div className="campo" style={{ flex: "0 1 16rem" }}>
                    <input
                      placeholder="Placa, número de unidad…"
                      value={busqueda}
                      onChange={(evento) => setBusqueda(evento.target.value)}
                    />
                  </div>
                </>
              }
              accionesDerecha={<ToggleVista vista={vista} onCambiar={setVista} />}
            />
          ) : (
            <Tabla<EncargadoRuta>
              id="rutas-encargados"
              columnas={columnasEncargados}
              filas={encargados}
              busqueda={busqueda}
              onCeldaEditada={manejarEdicionEncargado}
              onFilaDobleClic={setFormularioEncargado}
              controles={
                <>
                  <button
                    className="boton"
                    title="Ctrl+N"
                    onClick={() => setFormularioEncargado("crear")}
                  >
                    + Nuevo
                  </button>
                  <div className="campo" style={{ flex: "0 1 16rem" }}>
                    <input
                      placeholder="Código, nombre…"
                      value={busqueda}
                      onChange={(evento) => setBusqueda(evento.target.value)}
                    />
                  </div>
                </>
              }
              accionesDerecha={<ToggleVista vista={vista} onCambiar={setVista} />}
            />
          )}
        </div>
      </div>

      {formularioVehiculo && (
        <FormularioVehiculoRuta
          vehiculo={formularioVehiculo === "crear" ? undefined : formularioVehiculo}
          onCerrar={() => setFormularioVehiculo(null)}
          onGuardado={() => {
            setFormularioVehiculo(null);
            recargar();
          }}
        />
      )}

      {formularioEncargado && (
        <FormularioEncargadoRuta
          encargado={formularioEncargado === "crear" ? undefined : formularioEncargado}
          onCerrar={() => setFormularioEncargado(null)}
          onGuardado={() => {
            setFormularioEncargado(null);
            recargar();
          }}
        />
      )}
    </div>
  );
}
