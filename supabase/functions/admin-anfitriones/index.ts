import "jsr:@supabase/functions-js@2/edge-runtime.d.ts";
import { clienteServicio, correoAdminAutorizado } from "../_shared/admin.ts";
import { contrasenaAleatoria, generarCodigo } from "../_shared/anfitriones.ts";
import { json, leerCuerpo, preflight } from "../_shared/http.ts";
import { type FilaAnfitrion, procesar } from "./acciones.ts";

// Cuentas de anfitriones desde el panel web (ver la migración
// `activacion_de_anfitriones_desde_el_panel`):
//
//   { accion: "crear", correo, nombre }  → { correo, nombre, codigo, vence }
//   { accion: "restablecer", correo }    → { correo, nombre, codigo, vence }
//   { accion: "deshabilitar", correo }   → { correo, activo: false }
//   { accion: "habilitar", correo }      → { correo, activo: true }
//
// El código de activación se devuelve UNA sola vez; en la base solo queda su
// hash. Quien llama debe ser administrador del panel (`administradores_panel`).

/** "Para siempre" en el formato de GoTrue; se revierte con "none". */
const BLOQUEO_INDEFINIDO = "876000h";

Deno.serve(async (req: Request) => {
  const respuestaPreflight = preflight(req);
  if (respuestaPreflight) return respuestaPreflight;
  if (req.method !== "POST") return json({ error: "method_not_allowed" }, 405);

  const supabase = clienteServicio();
  const exigir = <T>(resultado: { data: T; error: { message: string } | null }): T => {
    if (resultado.error) throw new Error(resultado.error.message);
    return resultado.data;
  };

  const respuesta = await procesar(await leerCuerpo(req), {
    autorizar: () => correoAdminAutorizado(req, supabase),
    async esAdministrador(correo) {
      const fila = exigir(
        await supabase.from("administradores_panel").select("correo").eq("correo", correo).maybeSingle(),
      );
      return fila !== null;
    },
    async buscar(correo) {
      return exigir(
        await supabase.from("anfitriones").select("correo, nombre, activo, auth_user_id").eq("correo", correo)
          .maybeSingle(),
      ) as FilaAnfitrion | null;
    },
    async insertar(correo, nombre) {
      const { error } = await supabase.from("anfitriones").insert({ correo, nombre });
      if (error?.code === "23505") return false;
      if (error) throw new Error(error.message);
      return true;
    },
    async borrar(correo) {
      exigir(await supabase.from("anfitriones").delete().eq("correo", correo));
    },
    async asegurarCuenta(correo) {
      const { data, error } = await supabase.auth.admin.createUser({
        email: correo,
        password: contrasenaAleatoria(),
        email_confirm: true,
        app_metadata: { origen: "anfitriones" },
      });
      if (!error && data.user) return { id: data.user.id, creada: true };
      if (error?.code !== "email_exists") throw new Error(error?.message ?? "No se pudo crear la cuenta");
      // Ya tenía cuenta (p. ej. entraba con Google): se reutiliza.
      const id = exigir(await supabase.rpc("cuenta_auth_por_correo", { p_correo: correo })) as string | null;
      if (!id) throw new Error("La cuenta existe pero no se encontró");
      const actualizada = await supabase.auth.admin.updateUserById(id, { password: contrasenaAleatoria() });
      if (actualizada.error) throw new Error(actualizada.error.message);
      return { id, creada: false };
    },
    async borrarCuenta(id) {
      await supabase.auth.admin.deleteUser(id);
    },
    async enlazar(correo, id) {
      exigir(await supabase.from("anfitriones").update({ auth_user_id: id }).eq("correo", correo));
    },
    async contrasenaAleatoria(id) {
      const { error } = await supabase.auth.admin.updateUserById(id, { password: contrasenaAleatoria() });
      if (error) throw new Error(error.message);
    },
    async cerrarSesiones(id) {
      exigir(await supabase.rpc("cerrar_sesiones_de_cuenta", { p_usuario: id }));
    },
    async bloquear(id, bloquear) {
      const { error } = await supabase.auth.admin.updateUserById(id, {
        ban_duration: bloquear ? BLOQUEO_INDEFINIDO : "none",
      });
      if (error) throw new Error(error.message);
    },
    generarCodigo,
    async emitirCodigo(correo, codigo, admin, accion) {
      return exigir(
        await supabase.rpc("emitir_codigo_anfitrion", {
          p_correo: correo,
          p_codigo: codigo,
          p_emitido_por: admin,
          p_accion: accion,
        }),
      ) as string;
    },
    async cambiarEstado(correo, activo, admin) {
      return exigir(
        await supabase.rpc("cambiar_estado_anfitrion", { p_correo: correo, p_activo: activo, p_hecho_por: admin }),
      ) as string | null;
    },
  }).catch((fallo: unknown) => {
    console.error("admin-anfitriones:", fallo instanceof Error ? fallo.message : fallo);
    return { estado: 500 as const, cuerpo: { error: "error", detail: "No se pudo completar la acción." } };
  });

  if (respuesta.estado === 200 && "admin" in respuesta) {
    // Nunca el código: solo quién hizo qué.
    console.log(`admin-anfitriones: ${respuesta.admin} → ${respuesta.accion}`);
  }
  return json(respuesta.cuerpo, respuesta.estado);
});
