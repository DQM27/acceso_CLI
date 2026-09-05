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
        .select("correo, rol")
        .eq("correo", usuario.email)
        .maybeSingle();

      if (!vigente) return;

      if (errorConsulta || !admin) {
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
        rol: admin.rol as UsuarioSesion["rol"],
      });
      setCargando(false);
    }

    supabase.auth.getSession().then(({ data }) => autorizar(data.session?.user ?? null));

    // Sólo un login/logout de verdad amerita volver a "cargando" y re-
    // consultar `administradores_panel` -- Supabase también dispara este
    // evento con `TOKEN_REFRESHED` cada vez que la pestaña vuelve a estar
    // visible (o el token se renueva solo en segundo plano). Tratar eso
    // igual que un login real tiraba TODA la Shell a "cargando" (`Contenido`
    // devuelve `null` mientras tanto) sólo por cambiar de pestaña y volver
    // -- se sentía como que la página se reiniciaba, perdiendo la sección
    // en la que se estaba.
    const { data: suscripcion } = supabase.auth.onAuthStateChange((evento, session) => {
      if (evento !== "SIGNED_IN" && evento !== "SIGNED_OUT") return;
      setCargando(true);
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

  async function cerrarSesion() {
    await supabase.auth.signOut();
    setSesion(null);
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
