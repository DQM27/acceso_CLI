import { supabase } from "../lib/supabase";

/**
 * Espejo de `ingresos` en Supabase -- ver migración
 * `agrega_columnas_historial_a_ingresos`. Antes esa tabla sólo cacheaba
 * ingresos ABIERTOS para el cierre cruzado entre dispositivos del mismo
 * sitio; ahora que también manda el resto de columnas al cerrar, es un
 * historial real (multi-sitio, sin techo de tiempo -- decisión explícita,
 * ver conversación). RLS: sólo quien esté en `administradores_panel` puede
 * leer (`es_admin_global()`, migración
 * `administradores_panel_gestion_admin_global`) -- sin distinción de rol,
 * se eliminó `admin_regional` (ver migración `elimina_admin_regional`).
 */
export interface MovimientoHistorial {
  id: string;
  sitio_id: string;
  sitio_nombre: string | null;
  contratista_cedula: string | null;
  contratista_nombre: string;
  empresa_nombre: string | null;
  tipo_ingreso: string | null;
  medio_ingreso: string | null;
  gafete_numero: number | null;
  hora_entrada: string;
  hora_salida: string | null;
  usuario_entrada_nombre: string | null;
  usuario_salida_nombre: string | null;
}

interface FilaCruda {
  id: string;
  sitio_id: string;
  sitios: { nombre: string } | null;
  contratista_cedula: string | null;
  contratista_nombre: string;
  empresa_nombre: string | null;
  tipo_ingreso: string | null;
  medio_ingreso: string | null;
  gafete_numero: number | null;
  hora_entrada: string;
  hora_salida: string | null;
  usuario_entrada_nombre: string | null;
  usuario_salida_nombre: string | null;
}

export async function listarHistorial(desde?: string, hasta?: string): Promise<MovimientoHistorial[]> {
  let consulta = supabase
    .from("ingresos")
    .select(
      "id, sitio_id, contratista_cedula, contratista_nombre, empresa_nombre, tipo_ingreso, " +
        "medio_ingreso, gafete_numero, hora_entrada, hora_salida, usuario_entrada_nombre, " +
        "usuario_salida_nombre, sitios(nombre)",
    )
    .order("hora_entrada", { ascending: false });

  if (desde) consulta = consulta.gte("hora_entrada", desde);
  if (hasta) consulta = consulta.lte("hora_entrada", hasta);

  const { data, error } = await consulta.returns<FilaCruda[]>();
  if (error) throw new Error(error.message);

  return data.map(({ sitios, ...resto }) => ({ ...resto, sitio_nombre: sitios?.nombre ?? null }));
}
