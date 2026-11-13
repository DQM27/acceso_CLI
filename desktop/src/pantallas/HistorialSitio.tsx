import { useCallback, useRef, useState } from "react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import { idPorUuid } from "../componentes/Tabla.logica";
import type { TablaHandle } from "../componentes/Tabla";
import SelectorRangoFecha from "../componentes/SelectorRangoFecha";
import { textoRangoFecha } from "../componentes/SelectorRangoFecha.logica";
import BotonesExportacion from "../componentes/BotonesExportacion";
import { useCargaAlCambiar } from "../componentes/useCargaAlCambiar";
import { useBarraEstado } from "../contexto/BarraEstadoContexto";
import { textoConteo, useConteoVisible } from "../componentes/conteoTabla";
import type { PropsHistorialClase } from "./Historial.logica";

export interface ConfigHistorialSitio<T> {
  /** Lee el historial de la unidad en el rango (vacío = abierto). */
  listar: (desde?: string, hasta?: string) => Promise<T[]>;
  /** Estable (fuera del componente): ver el `useMemo` de `columnas` en Activos. */
  columnas: ColDef<T>[];
  /** Id de la grilla: clave del layout guardado de cada usuario. */
  idGrilla: string;
  placeholderBusqueda: string;
  nombreArchivo: string;
  tituloExportacion: string;
}

/**
 * Historiales de la unidad que la PC sólo copia de la nube (proveedores,
 * visitas por correo, gafetes provisionales KOF): buscador, rango de fechas,
 * exportación y la grilla. Las tres pantallas eran la misma copiada, sólo
 * cambiaban la consulta, las columnas y los textos (auditoría 2026-11-13).
 * Contratistas tiene su propia pantalla: junta lo local con la nube.
 */
export default function HistorialSitio<T extends { uuid: string }>({
  selector,
  desde,
  hasta,
  onRango,
  listar,
  columnas,
  idGrilla,
  placeholderBusqueda,
  nombreArchivo,
  tituloExportacion,
}: PropsHistorialClase & ConfigHistorialSitio<T>) {
  const [filas, setFilas] = useState<T[]>([]);
  const [cargando, setCargando] = useState(true);
  const [busqueda, setBusqueda] = useState("");
  const tablaRef = useRef<TablaHandle<T>>(null);

  const [conteo, alContar] = useConteoVisible();

  useBarraEstado(cargando ? "Cargando…" : textoConteo(conteo, filas.length, "movimiento(s)"));

  const recargar = useCallback(
    async (estaVigente: () => boolean = () => true) => {
      setCargando(true);
      try {
        const items = await listar(desde || undefined, hasta || undefined);
        if (estaVigente()) setFilas(items);
      } finally {
        if (estaVigente()) setCargando(false);
      }
    },
    [listar, desde, hasta],
  );
  useCargaAlCambiar(recargar, true);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="pantalla-cuerpo" style={{ minHeight: 0, flex: 1 }}>
        <div style={{ flex: 1, minHeight: 0 }}>
          <Tabla<T>
            ref={tablaRef}
            onConteoVisible={alContar}
            cargando={cargando}
            filtrosPorColumna
            id={idGrilla}
            idFila={idPorUuid}
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            controles={
              <div className="campo" style={{ flex: "0 1 16rem" }}>
                <input
                  placeholder={placeholderBusqueda}
                  value={busqueda}
                  onChange={(evento) => setBusqueda(evento.target.value)}
                />
              </div>
            }
            accionesCentro={selector}
            accionesDerecha={
              <>
                <SelectorRangoFecha desde={desde} hasta={hasta} onAplicar={onRango} />
                <BotonesExportacion
                  tablaRef={tablaRef}
                  nombreArchivo={nombreArchivo}
                  titulo={tituloExportacion}
                  filtroDescripcion={`Filtro: ${textoRangoFecha(desde, hasta)}`}
                />
              </>
            }
          />
        </div>
      </div>
    </div>
  );
}
