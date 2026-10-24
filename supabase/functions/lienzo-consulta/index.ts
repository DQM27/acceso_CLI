import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { json, leerCuerpo, preflight } from "../_shared/http.ts";
import { ejecutarConsulta, guardarAlerta } from "../_shared/lienzo/base.ts";
import { revisarSql } from "../_shared/lienzo/sql.ts";

// Ejecuta la consulta (fuente "consulta") de un lienzo armado por el asistente.
// El panel la llama con la sesión del administrador cada vez que dibuja el
// lienzo; no usa el modelo (no cuesta nada).
//
// Capas: 1) sólo administradores del panel; 2) el SQL se revisa con el parser
// (una sola consulta de lectura, sólo vistas de `ia`); 3) `ia_consultar` lo
// corre en solo lectura con el rol que corresponde a la cuenta. Un SQL que
// intenta salirse de `ia` queda como alerta para el dueño.
//
// Body: { sql }. Responde { columnas, filas, truncado } o { error, detail }.

const LIMITE_FILAS = 2000;

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  const correo = await correoAdminAutorizado(req, supabase);
  if (!correo) return json({ error: "no_autorizado", detail: "Sólo los administradores del panel pueden consultar datos." }, 401);

  const cuerpo = await leerCuerpo(req);
  const sql = typeof cuerpo?.sql === "string" ? cuerpo.sql : "";
  if (!sql.trim()) return json({ error: "pedido_invalido", detail: "Falta la consulta." }, 400);

  const revision = revisarSql(sql);
  if (!revision.ok) {
    if (revision.peligroso) {
      await guardarAlerta(supabase, {
        correo,
        origen: "parser",
        motivo: revision.error,
        mensaje: "(consulta enviada desde el panel, sin pasar por el chat)",
        detalle: sql,
      });
    }
    return json({ error: "consulta_no_permitida", detail: "Esa consulta no está permitida." }, 400);
  }

  const resultado = await ejecutarConsulta(supabase, correo, revision.sql, LIMITE_FILAS);
  if (!resultado.ok) {
    console.error("lienzo-consulta: la base rechazó la consulta:", resultado.error);
    return json({ error: "consulta_fallida", detail: "La consulta no se pudo ejecutar." }, 400);
  }
  return json({ columnas: resultado.columnas, filas: resultado.filas, truncado: resultado.truncado });
});
