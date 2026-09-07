import { createContext, useContext, useEffect, useState } from "react";
import type { ReactNode } from "react";
import type { User } from "@supabase/supabase-js";
import { supabase } from "../lib/supabase";
import type { UsuarioSesion } from "../api";

interface EstadoAuth {
  sesion: UsuarioSesion | null;
  cargando: boolean;
  /** No confundir con "credenciales inválidas" — Google ya autenticó a esta
   * persona; esto significa que su correo no está en
   * `administradores_panel` (ver esa migración). */
  error: string | null;
  iniciarSesionConGoogle: () => Promise<void>;
  cerrarSesion: () => Promise<void>;
}

const AuthContexto = createContext<EstadoAuth | null>(null);

/**
 * "Iniciar sesión con Google" sólo prueba identidad -- la autorización real
 * (¿puede esta persona entrar al panel?) la decide `administradores_panel`
 * en Postgres, consultada acá mismo tras cada cambio de sesión. Alguien con
 * cuenta de Google válida pero sin fila en esa tabla queda deslogueado de
 * inmediato, con `error` explicando por qué -- ver
 * docs/plan-panel-administrativo-web.md, sección "Decisión de auth".
 */
export function AuthProvider({ children }: { children: ReactNode }) {
  const [sesion, setSesion] = useState<UsuarioSesion | null>(null);
  const [cargando, setCargando] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let vigente = true;

    async function autorizar(usuario: User | null) {
      if (!usuario?.email) {
        if (vigente) {
          setSesion(null);
          setCargando(false);
        }
        return;
      }

      const { data: admin, error: errorConsulta } = await supabase
        .from("administradores_panel")
        .select("correo")
        .eq("correo", usuario.email)
        .maybeSingle();

      if (!vigente) return;

      // Error de red/consulta (timeout, Postgres caído un instante) NO es
      // lo mismo que "no está en administradores_panel" -- antes los dos
      // casos deslogueaban igual, así que un blip de conectividad durante
      // un re-chequeo en segundo plano (cambio de foco de pestaña, refresh
      // de token -- ver el comentario grande más abajo, esto pasa seguido)
      // podía sacar a un admin real de una sesión que ya tenía andando,
      // con un mensaje que además le hacía dudar si tenía acceso. Acá no
      // se toca `sesion` ni se cierra sesión -- si ya había una sesión
      // válida, se mantiene tal cual hasta el próximo re-chequeo exitoso.
      if (errorConsulta) {
        setError(
          "No se pudo confirmar tu acceso al panel (falla de conexión) -- probá iniciar sesión de nuevo.",
        );
        setCargando(false);
        return;
      }

      if (!admin) {
        setError(
          `La cuenta de Google "${usuario.email}" inició sesión, pero no está autorizada para este panel.`,
        );
        setSesion(null);
        setCargando(false);
        await supabase.auth.signOut();
        return;
      }

      setError(null);
      setSesion({
        nombre: (usuario.user_metadata?.full_name as string | undefined) ?? usuario.email,
        correo: usuario.email,
      });
      setCargando(false);
    }

    supabase.auth.getSession().then(({ data }) => autorizar(data.session?.user ?? null));

    // `cargando` sólo vuelve a `true` en el `useState(true)` inicial de
    // arriba -- nunca se vuelve a tocar acá a propósito. Supabase dispara
    // `onAuthStateChange` (con `SIGNED_IN`, `TOKEN_REFRESHED` o incluso de
    // nuevo `SIGNED_IN` según la versión) cada vez que la pestaña recupera
    // el foco, no sólo en un login real. Poner `cargando` en `true` en cada
    // uno de esos hacía que `Contenido` devolviera `null` un instante,
    // desmontando TODA la Shell -- se sentía como que la página se
    // reiniciaba (perdiendo la sección en la que se estaba) sólo por
    // cambiar de pestaña y volver. Acá `autorizar` sigue re-chequeando
    // `administradores_panel` igual (por si el acceso cambió mientras
    // tanto), pero actualiza `sesion` en silencio sin desmontar nada --
    // React no resetea el estado de un componente que sigue montado en el
    // mismo lugar del árbol sólo porque sus props cambiaron.
    const { data: suscripcion } = supabase.auth.onAuthStateChange((_evento, session) => {
      autorizar(session?.user ?? null);
    });

    return () => {
      vigente = false;
      suscripcion.subscription.unsubscribe();
    };
  }, []);

  async function iniciarSesionConGoogle() {
    setError(null);
    await supabase.auth.signInWithOAuth({ provider: "google" });
  }

  /** Cierra la sesión de Google (Supabase) y, además, la de Cloudflare
   * Access -- son dos capas independientes con cookies propias; sin este
   * segundo paso, la persona vuelve a ver el botón de Google directo
   * porque la cookie `CF_AppSession` (24h) sigue viva y Access ni siquiera
   * vuelve a pedir el código. `/cdn-cgi/access/logout` es el endpoint que
   * Cloudflare expone en todo dominio protegido para revocar esa cookie.
   * Recarga la página entera al final para que el próximo request dispare
   * el desafío de Access de nuevo (no alcanza con limpiar el estado de
   * React, la próxima carga la sirve el edge, no esta SPA). */
  async function cerrarSesion() {
    await supabase.auth.signOut();
    setSesion(null);
    try {
      await fetch("/cdn-cgi/access/logout", { credentials: "include" });
    } catch {
      // Si esto falla (ej. no está detrás de Access en dev local), la
      // sesión de Google ya se cerró igual -- no es motivo para romper el
      // flujo normal de logout.
    }
    window.location.href = "/";
  }

  return (
    <AuthContexto.Provider value={{ sesion, cargando, error, iniciarSesionConGoogle, cerrarSesion }}>
      {children}
    </AuthContexto.Provider>
  );
}

export function useAuth(): EstadoAuth {
  const contexto = useContext(AuthContexto);
  if (!contexto) throw new Error("useAuth debe usarse dentro de AuthProvider");
  return contexto;
}
