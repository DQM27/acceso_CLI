import type { ColDef } from "ag-grid-community";
import InterruptorCelda from "../componentes/InterruptorCelda";
import { textoTipoIngreso } from "../api";
import type { ContratistaResumen, RolUsuario } from "../api";
import { textoFechaDDMMYYYY } from "../tiempo";

// "es de ruta"/"tiene acceso" se pueden tocar directo desde la grilla (ambos
// booleanos, bajo riesgo) — el resto (cédula, nombre, empresa, tipo, PRAIND)
// pasa por FormularioContratista (doble click en una fila para editar, botón
// "+ Nuevo" para dar de alta). Sin formulario de filtros a medida: el
// buscador de arriba (`busqueda`, quickFilterText de AG Grid) más los
// filtros nativos por columna (`filtrosPorColumna`, mismo enfoque que
// Historial) alcanzan — la grilla carga el universo completo una sola vez
// (ver `buscarContratistas`) y el resto pasa del lado del cliente. Los dos
// booleanos no llevan filtro de columna (`filter: false`): AG Grid Community
// no tiene un filtro booleano nativo decente, y ya se ven/tocan directo con
// el switch.
//
// "Acceso" (`tiene_acceso`) se oculta para Operador (ver `Contratistas`,
// más abajo) -- 2026-09-06: el cambio de este estado se delega a los
// admins (panel web), un Operador viéndola sin poder usarla para nada
// sólo generaba confusión. El núcleo todavía no lo exige (cualquier rol
// autenticado puede llamar `actualizarContratista` igual) -- esto es sólo
// la UI, no un candado de permisos nuevo.
export function columnasPara(actorRol: RolUsuario): ColDef<ContratistaResumen>[] {
  const columnas: ColDef<ContratistaResumen>[] = [
    { field: "cedula", headerName: "Cédula", flex: 1.4, minWidth: 140, cellStyle: { textAlign: "left" } },
    { field: "nombre", headerName: "Nombre", flex: 1.6, minWidth: 170, cellStyle: { textAlign: "left" } },
    { field: "empresa_nombre", headerName: "Empresa", flex: 1.4, minWidth: 140 },
    {
      field: "tipo_ingreso",
      headerName: "Tipo",
      flex: 1.2,
      minWidth: 120,
      // Texto visible también para filtrar/ordenar ("IN HOUSE", no "InHouse").
      valueGetter: (p) => textoTipoIngreso(p.data?.tipo_ingreso ?? null),
    },
    {
      field: "fecha_vencimiento_praind",
      type: "fecha",
      headerName: "PRAIND vence",
      flex: 1.4,
      minWidth: 140,
      valueFormatter: (p) => (p.value ? textoFechaDDMMYYYY(p.value) : ""),
    },
    {
      field: "es_personal_ruta",
      headerName: "Personal de ruta",
      // 140 truncaba el encabezado ("PERSONAL DE …") — 170 es lo que
      // necesita "PERSONAL DE RUTA" para no cortarse.
      flex: 1.7,
      minWidth: 170,
      cellRenderer: InterruptorCelda,
      filter: false,
    },
  ];
  if (actorRol !== "Operador") {
    columnas.push({
      field: "tiene_acceso",
      headerName: "Acceso",
      flex: 1,
      minWidth: 100,
      cellRenderer: InterruptorCelda,
      cellRendererParams: { critico: true },
      filter: false,
    });
  }
  return columnas;
}
