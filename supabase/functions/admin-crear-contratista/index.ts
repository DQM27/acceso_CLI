import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { json, leerCuerpo, preflight } from "../_shared/http.ts";
import { hoyCostaRica } from "../_shared/reglas.ts";
import { procesarAlta } from "./alta.ts";

// Alta de un contratista desde el panel web. Reemplaza a la función SQL
// `panel_crear_contratista`, que repetía las reglas en SQL y se había
// desviado del núcleo (seguía aceptando "POR CORREO", retirado el
// 2026-10-03). Ahora las reglas de criterio son las del núcleo, compiladas
// a WebAssembly (`_shared/reglas.ts`), las mismas que usa el formulario del
// panel para avisar antes de enviar. Ver docs/arquitectura/reglas-compartidas.md.
//
// 1. Quien llama es administrador del panel (`administradores_panel`).
// 2. Reglas de criterio del núcleo (cédula, nombre, tipo, PRAIND).
// 3. La empresa existe.
// 4. Guarda con la cuenta de servicio; la cédula repetida la frena el índice
//    único `contratistas_cedula_normalizada_key`.
//
// Body: { cedula, nombre, empresa_id, tipo_ingreso, fecha_vencimiento_praind,
//         con_acceso }. Responde la fila creada, o { error, detail } con el
//         mismo texto que muestran las apps.

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  const correo = await correoAdminAutorizado(req, supabase);
  if (!correo) return json({ error: "unauthorized", detail: "No tiene permiso para crear contratistas." }, 401);

  const respuesta = await procesarAlta(await leerCuerpo(req), hoyCostaRica(), {
    async buscarEmpresa(id) {
      const { data, error } = await supabase.from("empresas").select("id, nombre").eq("id", id).maybeSingle();
      if (error) throw new Error(error.message);
      return data;
    },
    async insertar(contratista, empresa) {
      const { data, error } = await supabase
        .from("contratistas")
        .insert({
          // `id` no tiene valor por defecto: cada equipo genera el suyo, y el panel también.
          id: crypto.randomUUID(),
          dispositivo_origen_id: null,
          nombre: contratista.nombre,
          identificacion: contratista.cedula,
          activo: contratista.tiene_acceso,
          empresa_id: empresa.id,
          empresa_nombre: empresa.nombre,
          tipo_ingreso: contratista.tipo_ingreso,
          fecha_vencimiento_praind: contratista.fecha_vencimiento_praind,
          es_personal_ruta: contratista.es_personal_ruta,
        })
        .select("*")
        .single();
      if (error) {
        if (error.code === "23505") return { ok: false, motivo: "cedula_duplicada" };
        console.error("admin-crear-contratista: no se pudo guardar:", error.message);
        return { ok: false, motivo: "error", detalle: error.message };
      }
      return { ok: true, fila: data };
    },
  });

  if (respuesta.estado === 200) {
    console.log(`admin-crear-contratista: ${correo} creó un contratista`);
  }
  return json(respuesta.cuerpo, respuesta.estado);
});
