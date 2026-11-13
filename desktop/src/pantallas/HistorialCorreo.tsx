import { listarHistorialIngresosCorreoSitio } from "../api/correo";
import type { HistorialIngresoCorreoRemoto } from "../api/correo";
import type { PropsHistorialClase } from "./Historial.logica";
import HistorialSitio from "./HistorialSitio";
import { columnasIngresoSitio } from "./HistorialSitio.columnas";

const COLUMNAS = columnasIngresoSitio<HistorialIngresoCorreoRemoto>({
  field: "motivo",
  headerName: "Motivo",
  flex: 1.2,
  minWidth: 140,
  valueFormatter: (p) => p.value ?? "—",
});

/** Historial de visitas por correo: lee `historial_ingresos_correo_sitio`,
 * toda la unidad. Las visitas agendadas siguen en su propia pantalla. */
export default function HistorialCorreo(props: PropsHistorialClase) {
  return (
    <HistorialSitio<HistorialIngresoCorreoRemoto>
      {...props}
      listar={listarHistorialIngresosCorreoSitio}
      columnas={COLUMNAS}
      idGrilla="correo-historial"
      placeholderBusqueda="Cédula, nombre o motivo…"
      nombreArchivo="historial-correo"
      tituloExportacion="Historial de Visitas por correo"
    />
  );
}
