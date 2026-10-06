import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { clienteServicio } from "../_shared/admin.ts";
import { json } from "../_shared/http.ts";
import { type Cuenta, type FilaAnfitrion, ORIGEN, planificar } from "./plan.ts";

// Mantiene las cuentas de Supabase Auth de los anfitriones al día con la
// tabla `anfitriones`, que es donde se da el alta y la baja por SQL (ver la
// migración `alta_de_anfitriones_por_sql`). La dispara el trigger
// `anfitriones_sincronizar_cuentas`; ignora el cuerpo y relee todo, así que
// llamarla de más nunca hace daño y un aviso perdido lo corrige el siguiente.
// Qué decide está en plan.ts.
//
// Las cuentas nuevas quedan con el correo confirmado, SIN contraseña y sin
// mandar ningún correo: la persona define su contraseña desde la web con el
// código de 6 dígitos ("¿primer ingreso?").
//
// Pública para la plataforma (`verify_jwt = false`): la autorización real
// es el header `x-webhook-secret`, que se compara contra Vault con
// `secreto_cuentas_anfitriones_valido` (solo `service_role`). La respuesta
// devuelve conteos, nunca correos.

/** "Para siempre" en el formato de GoTrue; se revierte con "none". */
const BLOQUEO_INDEFINIDO = "876000h";
const POR_PAGINA = 1000;

async function todasLasCuentas(admin: ReturnType<typeof clienteServicio>): Promise<Cuenta[]> {
  const cuentas: Cuenta[] = [];
  for (let pagina = 1; ; pagina++) {
    const { data, error } = await admin.auth.admin.listUsers({ page: pagina, perPage: POR_PAGINA });
    if (error) throw error;
    cuentas.push(...(data.users as Cuenta[]));
    if (data.users.length < POR_PAGINA) return cuentas;
  }
}

Deno.serve(async (req: Request) => {
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const admin = clienteServicio();
  const { data: valido, error: errorSecreto } = await admin.rpc("secreto_cuentas_anfitriones_valido", {
    p_secreto: req.headers.get("x-webhook-secret") ?? "",
  });
  if (errorSecreto) return json({ error: "verificacion_fallida" }, 500);
  if (valido !== true) return json({ error: "unauthorized" }, 401);

  const [anfitriones, administradores] = await Promise.all([
    admin.from("anfitriones").select("correo, activo"),
    admin.from("administradores_panel").select("correo"),
  ]);
  if (anfitriones.error || administradores.error) return json({ error: "lectura_fallida" }, 500);

  let cuentas: Cuenta[];
  try {
    cuentas = await todasLasCuentas(admin);
  } catch {
    return json({ error: "auth_fallido" }, 500);
  }

  const acciones = planificar(
    anfitriones.data as FilaAnfitrion[],
    (administradores.data ?? []).map((fila) => fila.correo as string),
    cuentas,
  );

  const resumen = { creadas: 0, bloqueadas: 0, desbloqueadas: 0, errores: 0 };
  for (const accion of acciones) {
    if (accion.tipo === "crear") {
      const { error } = await admin.auth.admin.createUser({
        email: accion.correo,
        email_confirm: true,
        app_metadata: { origen: ORIGEN },
      });
      // `email_exists`: otra ejecución simultánea ya la creó.
      if (!error) resumen.creadas++;
      else if (error.code !== "email_exists") resumen.errores++;
    } else {
      const bloquear = accion.tipo === "bloquear";
      const { error } = await admin.auth.admin.updateUserById(accion.id, {
        ban_duration: bloquear ? BLOQUEO_INDEFINIDO : "none",
        app_metadata: accion.app_metadata,
      });
      if (error) resumen.errores++;
      else if (bloquear) resumen.bloqueadas++;
      else resumen.desbloqueadas++;
    }
  }

  return json({ ok: resumen.errores === 0, ...resumen }, resumen.errores === 0 ? 200 : 207);
});
