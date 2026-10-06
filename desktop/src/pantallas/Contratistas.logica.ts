import type { ColDef } from "ag-grid-community";
import InterruptorCelda from "../componentes/InterruptorCelda";
import { textoTipoIngreso } from "../api";
import type { ContratistaResumen, EstadoAccesoLista, RolUsuario } from "../api";
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
/** Qué tan grave es el estado: define el color de la celda. */
export type NivelEstado = "ok" | "aviso" | "bloqueo";

/** Mismos textos que el panel web (`web/src/pantallas/Contratistas.logica.ts`):
 * las dos interfaces dicen lo mismo de cada contratista. */
export const ESTADOS_ACCESO: Record<EstadoAccesoLista, { texto: string; nivel: NivelEstado }> = {
  PERMITIDO: { texto: "Puede entrar", nivel: "ok" },
  PERMITIDO_CON_ADVERTENCIA: { texto: "Puede entrar (PRAIND por vencer)", nivel: "aviso" },
  PRAIND_VENCIDO: { texto: "PRAIND vencida", nivel: "bloqueo" },
  PRAIND_NO_REGISTRADO: { texto: "Sin PRAIND registrada", nivel: "bloqueo" },
  SIN_ACCESO: { texto: "Acceso denegado", nivel: "bloqueo" },
  EMPRESA_INACTIVA: { texto: "Empresa inactiva", nivel: "bloqueo" },
};

const COLOR_NIVEL: Record<NivelEstado, string> = {
  ok: "var(--exito)",
  aviso: "var(--advertencia)",
  bloqueo: "var(--error)",
};

function dias(n: number): string {
  return n === 1 ? "1 día" : `${n} días`;
}

/** Texto de la columna "Estado": el resultado de las reglas y, si la PRAIND
 * está por vencer o vencida, cuántos días. */
export function textoEstadoAcceso(
  fila: Pick<ContratistaResumen, "estado_acceso" | "dias_para_vencer_praind">,
): string {
  if (!fila.estado_acceso) return "";
  const restantes = fila.dias_para_vencer_praind;
  if (fila.estado_acceso === "PERMITIDO_CON_ADVERTENCIA" && restantes !== null)
    return restantes === 0 ? "Puede entrar (PRAIND vence hoy)" : `Puede entrar (PRAIND vence en ${dias(restantes)})`;
  if (fila.estado_acceso === "PRAIND_VENCIDO" && restantes !== null && restantes < 0)
    return `PRAIND vencida hace ${dias(-restantes)}`;
  return ESTADOS_ACCESO[fila.estado_acceso].texto;
}

export function colorEstadoAcceso(estado: EstadoAccesoLista | null | undefined): string | undefined {
  return estado ? COLOR_NIVEL[ESTADOS_ACCESO[estado].nivel] : undefined;
}

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
      colId: "estado_acceso",
      headerName: "Estado",
      flex: 2,
      minWidth: 210,
      valueGetter: (p) => (p.data ? textoEstadoAcceso(p.data) : ""),
      cellStyle: (p) => {
        const color = colorEstadoAcceso(p.data?.estado_acceso);
        return color ? { color, fontWeight: 600 } : null;
      },
    },
    {
      field: "tiene_ingreso_activo",
      headerName: "Adentro",
      flex: 0.9,
      minWidth: 100,
      // Texto para que también se pueda filtrar ("Sí").
      valueGetter: (p) => (p.data?.tiene_ingreso_activo ? "Sí" : ""),
      cellStyle: { fontWeight: 600 },
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
