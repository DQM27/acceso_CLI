import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { clienteServicio } from "../_shared/admin.ts";
import { json, leerCuerpo, preflight } from "../_shared/http.ts";
import { activar } from "./activacion.ts";

// La persona activa su cuenta de anfitrión (primer ingreso o después de un
// "restablecer" del panel) con correo + código + su contraseña nueva. Ver la
// migración `activacion_de_anfitriones_desde_el_panel`.
//
// Pública a propósito (`verify_jwt = false`): quien llega todavía no tiene
// sesión. La protección es el código: hash bcrypt, vence a las 72 h y se
// agota a los 5 intentos fallidos (lo cuenta la base, no esta función). La
// respuesta a cualquier rechazo del código es la misma.
//
// Body: { correo, codigo, contrasena } → { ok: true }, o { error, detail }.

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  try {
    const respuesta = await activar(await leerCuerpo(req), {
      async verificar(correo, codigo) {
        const { data, error } = await supabase.rpc("verificar_codigo_anfitrion", {
          p_correo: correo,
          p_codigo: codigo,
        });
        if (error) throw new Error(error.message);
        return data as string | null;
      },
      async ponerContrasena(id, contrasena) {
        const { error } = await supabase.auth.admin.updateUserById(id, { password: contrasena });
        if (!error) return "ok";
        if (error.code === "weak_password") return "debil";
        throw new Error(error.message);
      },
      async consumir(correo) {
        const { error } = await supabase.rpc("consumir_codigo_anfitrion", { p_correo: correo });
        if (error) throw new Error(error.message);
      },
      async cerrarSesiones(id) {
        const { error } = await supabase.rpc("cerrar_sesiones_de_cuenta", { p_usuario: id });
        if (error) throw new Error(error.message);
      },
    });
    if (respuesta.estado === 200) console.log("anfitrion-activar: cuenta activada");
    return json(respuesta.cuerpo, respuesta.estado);
  } catch (fallo) {
    console.error("anfitrion-activar:", fallo instanceof Error ? fallo.message : fallo);
    return json({ error: "error", detail: "No se pudo activar la cuenta. Intente de nuevo." }, 500);
  }
});
