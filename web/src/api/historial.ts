import { z } from "zod";
import { supabase } from "../lib/supabase";
import { inicioDiaCostaRicaUtc, inicioDiaSiguienteCostaRicaUtc } from "../tiempo";

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
  // "pc"/"mobile"/"visor" (`dispositivos.tipo`) -- null si el dispositivo
  // de entrada fue borrado, o para filas viejas sin dispositivo_entrada_id.
  dispositivo_entrada_tipo: string | null;
}

// Valida en runtime la forma real de lo que devuelve Supabase -- ver el
// mismo criterio en contratistas.ts/usuarios.ts. `z.infer` reemplaza a la
// interfaz `FilaCruda` que había antes, para no mantener dos fuentes de
// verdad del mismo shape.
const filaCrudaEsquema = z.object({
  id: z.string(),
  sitio_id: z.string(),
  sitios: z.object({ nombre: z.string() }).nullable(),
  contratista_cedula: z.string().nullable(),
  contratista_nombre: z.string(),
  empresa_nombre: z.string().nullable(),
  tipo_ingreso: z.string().nullable(),
  medio_ingreso: z.string().nullable(),
  gafete_numero: z.number().nullable(),
  hora_entrada: z.string(),
  hora_salida: z.string().nullable(),
  usuario_entrada_nombre: z.string().nullable(),
  usuario_salida_nombre: z.string().nullable(),
  dispositivo_entrada: z.object({ tipo: z.string() }).nullable(),
});

export interface ResultadoHistorial {
  filas: MovimientoHistorial[];
  /** `true` si el rango pedido tiene más filas que `LIMITE_HISTORIAL` -- ver
   * esa constante. AG Grid corre en modo client-side (trae todo, filtra en
   * el navegador, ver `componentes/Tabla.tsx`); sin este tope, un rango
   * amplio (o el preset "Todo el historial", sin fecha) podía crecer sin
   * cota junto con el uso real del sistema. Mismo criterio que
   * `CargaCompleta.truncado` del núcleo Rust en la versión de escritorio
   * (`desktop/src/pantallas/Historial.tsx`) -- filas visibles acotadas,
   * exportar (Excel/PDF) sigue trayendo el rango completo sin este límite
   * (ver `exportarAExcel`/`exportarAPdf` en `pantallas/Historial.tsx`).
   */
  truncado: boolean;
}

// Bien por encima de cualquier volumen real de un rango de fechas típico
// (6 meses por defecto, ver `Historial.tsx`) -- es una válvula de
// seguridad, no una paginación real: mientras el volumen se mantenga
// razonable, nadie la nota.
const LIMITE_HISTORIAL = 20_000;

export async function listarHistorial(desde?: string, hasta?: string): Promise<ResultadoHistorial> {
  let consulta = supabase
    .from("ingresos")
    .select(
      "id, sitio_id, contratista_cedula, contratista_nombre, empresa_nombre, tipo_ingreso, " +
        "medio_ingreso, gafete_numero, hora_entrada, hora_salida, usuario_entrada_nombre, " +
        "usuario_salida_nombre, sitios(nombre), " +
        "dispositivo_entrada:dispositivos!ingresos_dispositivo_entrada_id_fkey(tipo)",
      { count: "exact" },
    )
    .order("hora_entrada", { ascending: false })
    .range(0, LIMITE_HISTORIAL - 1);

  // `desde`/`hasta` llegan como YMD del selector (día calendario en Costa
  // Rica, ver `SelectorRangoFecha`), pero `hora_entrada` es un `timestamptz`
  // en UTC -- compararlo contra el string crudo lo interpreta a medianoche
  // UTC (no Costa Rica) y, para `hasta`, deja afuera casi todo ese día (sólo
  // calificaría el instante exacto de esa medianoche). Con un rango amplio
  // el corte pasaba desapercibido; con "Hoy"/"Ayer" (mismo día en desde y
  // hasta) el rango resultante quedaba prácticamente vacío siempre. Mismo
  // criterio que `rango_utc` en
  // `desktop/src-tauri/src/comandos/historial.rs`: `hasta` es el inicio del
  // día SIGUIENTE, límite exclusivo.
  if (desde) consulta = consulta.gte("hora_entrada", inicioDiaCostaRicaUtc(desde));
  if (hasta) consulta = consulta.lt("hora_entrada", inicioDiaSiguienteCostaRicaUtc(hasta));

  const { data: crudo, error, count } = await consulta;
  if (error) throw new Error(error.message);
  const data = z.array(filaCrudaEsquema).parse(crudo);

  return {
    filas: data.map(({ sitios, dispositivo_entrada, ...resto }) => ({
      ...resto,
      sitio_nombre: sitios?.nombre ?? null,
      dispositivo_entrada_tipo: dispositivo_entrada?.tipo ?? null,
    })),
    truncado: count !== null && count > data.length,
  };
}
