import { listarHistorialIngresosProveedorSitio } from "../api/proveedores";
import type { HistorialIngresoProveedorRemoto } from "../api/proveedores";
import type { PropsHistorialClase } from "./Historial.logica";
import HistorialSitio from "./HistorialSitio";
import { columnasIngresoSitio } from "./HistorialSitio.columnas";

const COLUMNAS = columnasIngresoSitio<HistorialIngresoProveedorRemoto>({
  field: "empresa_nombre",
  headerName: "Empresa",
  flex: 1.2,
  minWidth: 140,
  valueFormatter: (p) => p.value ?? "—",
});

/** Historial de proveedores: lee `historial_ingresos_proveedor_sitio`, una
 * caché que sólo sincroniza la PC (el celular no la trae a propósito). */
export default function HistorialProveedores(props: PropsHistorialClase) {
  return (
    <HistorialSitio<HistorialIngresoProveedorRemoto>
      {...props}
      listar={listarHistorialIngresosProveedorSitio}
      columnas={COLUMNAS}
      idGrilla="proveedores-historial"
      placeholderBusqueda="Cédula, nombre, empresa…"
      nombreArchivo="historial-proveedores"
      tituloExportacion="Historial de Proveedores"
    />
  );
}
