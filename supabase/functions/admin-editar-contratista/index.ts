import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { columnasContratista } from "../_shared/contratistas.ts";
import { json, leerCuerpo, preflight } from "../_shared/http.ts";
import { hoyCostaRica } from "../_shared/reglas.ts";
import { procesarEdicion } from "./edicion.ts";

// Edición de un contratista desde el panel web, con las reglas del núcleo
// (WebAssembly, `_shared/reglas.ts`), las mismas que usa el formulario del
// panel para avisar antes de enviar y que escritorio y teléfono aplican al
// editar. Ver docs/arquitectura/reglas-compartidas.md.
//
// 1. Quien llama es administrador del panel (`administradores_panel`).
// 2. Reglas de criterio del núcleo, con lo que el contratista tenía guardado.
// 3. La empresa existe.
// 4. No se cambia la cédula de quien está adentro.
// 5. Guarda con la cuenta de servicio; la cédula repetida la frena el índice
//    único `contratistas_cedula_normalizada_key`. El trigger
//    `contratistas_emitir_cambio_nube` avisa a los equipos.
//
// Body: { id, cedula, nombre, empresa_id, tipo_ingreso,
//         fecha_vencimiento_praind, es_personal_ruta, con_acceso }.
// Responde la fila guardada, o { error, detail } con el mismo texto que
// muestran las apps.

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  const respuesta = await procesarEdicion(await leerCuerpo(req), hoyCostaRica(), {
    autorizar: () => correoAdminAutorizado(req, supabase),
    async buscarContratista(id) {
      const { data, error } = await supabase
        .from("contratistas")
        .select("id, identificacion, tipo_ingreso, es_personal_ruta, fecha_vencimiento_praind")
        .eq("id", id)
        .maybeSingle();
      if (error) throw new Error(error.message);
      return data;
    },
    async buscarEmpresa(id) {
      const { data, error } = await supabase.from("empresas").select("id, nombre").eq("id", id).maybeSingle();
      if (error) throw new Error(error.message);
      return data;
    },
    async estaAdentro(contratista) {
      // Por id (lo normal) o por la cédula guardada (ingresos viejos sin id).
      const { count, error } = await supabase
        .from("ingresos")
        .select("id", { count: "exact", head: true })
        .is("hora_salida", null)
        .or(`contratista_id.eq.${contratista.id},contratista_cedula.eq.${contratista.identificacion}`);
      if (error) throw new Error(error.message);
      return (count ?? 0) > 0;
    },
    async actualizar(id, contratista, empresa) {
      const { data, error } = await supabase
        .from("contratistas")
        .update(columnasContratista(contratista, empresa))
        .eq("id", id)
        .select("*")
        .single();
      if (error) {
        if (error.code === "23505") return { ok: false, motivo: "cedula_duplicada" };
        console.error("admin-editar-contratista: no se pudo guardar:", error.message);
        return { ok: false, motivo: "error", detalle: error.message };
      }
      return { ok: true, fila: data };
    },
  });

  if (respuesta.estado === 200) {
    console.log(`admin-editar-contratista: ${respuesta.correo} editó un contratista`);
  }
  return json(respuesta.cuerpo, respuesta.estado);
});
