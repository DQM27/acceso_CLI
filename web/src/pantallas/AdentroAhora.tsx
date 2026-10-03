import { useMemo, useState } from "react";
import type { ColDef } from "ag-grid-community";
import Tabla from "../componentes/Tabla";
import { useLista } from "../componentes/useLista";
import {
  HORAS_ALERTA_ADENTRO,
  TEXTO_TIPO_ADENTRO,
  listarAdentroAhora,
  llevaDemasiado,
  textoTiempoAdentro,
} from "../api/adentro";
import type { PersonaAdentro, TipoAdentro } from "../api/adentro";
import { fechaLocalYMD, textoFechaDDMMYYYY, textoHora } from "../tiempo";

const TIPOS: TipoAdentro[] = ["CONTRATISTA", "PROVEEDOR", "PROVISIONAL_KOF", "POR_CORREO"];

/**
 * Quién está adentro ahora en cada unidad (contratistas, proveedores,
 * gafetes provisionales KOF e ingresos por correo sin salida). Sirve para el control diario, para
 * un conteo de emergencia y para ver salidas que se olvidaron marcar: quien
 * lleva más de `HORAS_ALERTA_ADENTRO` horas se resalta.
 *
 * Se refresca con cada cambio de `ingresos` (Realtime) y cada 30 s: proveedores,
 * préstamos KOF e ingresos por correo no están en la publicación de Realtime,
 * así que dependen del intervalo.
 */
export default function AdentroAhora() {
  const [busqueda, setBusqueda] = useState("");
  const { datos, cargando } = useLista(["adentro-ahora"], listarAdentroAhora, {
    intervaloMs: 30_000,
    tablas: "ingresos",
  });
  const filas = useMemo(() => datos ?? [], [datos]);

  const conteo = useMemo(() => {
    const porTipo = new Map<TipoAdentro, number>(TIPOS.map((tipo) => [tipo, 0]));
    for (const fila of filas) porTipo.set(fila.tipo, (porTipo.get(fila.tipo) ?? 0) + 1);
    return porTipo;
  }, [filas]);

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
        cellStyle: (p) =>
          p.data && llevaDemasiado(p.data.hora_entrada)
            ? { color: "var(--error)", fontWeight: 600 }
            : null,
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
            id="adentro-ahora"
            columnas={columnas}
            filas={filas}
            busqueda={busqueda}
            filtrosPorColumna
            idFila={(fila) => `${fila.tipo}:${fila.id}`}
            controles={
              <>
                <div className="campo flex-[0_1_16rem]">
                  <input
                    placeholder="Cédula, nombre, gafete…"
                    value={busqueda}
                    disabled={cargando}
                    onChange={(evento) => setBusqueda(evento.target.value)}
                  />
                </div>
                <span role="status" title={`En rojo: más de ${HORAS_ALERTA_ADENTRO} h adentro`}>
                  {filas.length.toLocaleString("es-CR")} adentro ·{" "}
                  {TIPOS.map((tipo) => `${TEXTO_TIPO_ADENTRO[tipo]}: ${conteo.get(tipo) ?? 0}`).join(" · ")}
                </span>
              </>
            }
          />
        </div>
      </div>
    </div>
  );
}
