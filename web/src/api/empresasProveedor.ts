import { z } from "../lib/validacion";
import { supabase } from "../lib/supabase";

/**
 * Catálogo de empresas de proveedor de todas las unidades, para ordenarlo:
 * lo que se crea en la portería queda "por revisar" hasta que el
 * administrador lo confirma o lo fusiona con la empresa que ya existía (era
 * la misma, mal escrita). Migración
 * `20261031120000_empresas_proveedor_revision_y_fusion`.
 */

const empresaProveedorEsquema = z.object({
  id: z.string(),
  sitio_id: z.string(),
  sitio_nombre: z.string(),
  nombre: z.string(),
  activa: z.boolean(),
  created_at: z.string(),
  revisada_en: z.string().nullable(),
  revisada_por: z.string().nullable(),
  fusionada_en: z.string().nullable(),
  fusionada_en_nombre: z.string().nullable(),
  creada_por_equipo: z.string().nullable(),
  ingresos: z.number(),
  ultimo_ingreso: z.string().nullable(),
});

export type EmpresaProveedorPanel = z.infer<typeof empresaProveedorEsquema>;

/** Catálogo chico (decenas por unidad): se trae entero, ordenado por nombre. */
export async function listarEmpresasProveedor(): Promise<
  EmpresaProveedorPanel[]
> {
  const { data, error } = await supabase
    .from("panel_empresas_proveedor")
    .select(
      "id, sitio_id, sitio_nombre, nombre, activa, created_at, revisada_en, revisada_por, " +
        "fusionada_en, fusionada_en_nombre, creada_por_equipo, ingresos, ultimo_ingreso",
    )
    .order("nombre");
  if (error) throw new Error(error.message);
  return z.array(empresaProveedorEsquema).parse(data);
}

/** Es una empresa nueva de verdad: sale de "por revisar". */
export async function confirmarEmpresaProveedor(id: string): Promise<void> {
  const { error } = await supabase.rpc("panel_confirmar_empresa_proveedor", {
    p_id: id,
  });
  if (error) throw new Error(error.message);
}

/** `origen` era la misma empresa que `destino`, mal escrita: queda marcada,
 * desactivada, y sus ingresos pasan a `destino`. */
export async function fusionarEmpresaProveedor(
  origen: string,
  destino: string,
): Promise<void> {
  const { error } = await supabase.rpc("panel_fusionar_empresa_proveedor", {
    p_origen: origen,
    p_destino: destino,
  });
  if (error) throw new Error(error.message);
}
