import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
} from "react";
import type { ReactNode } from "react";
import type { Session } from "@supabase/supabase-js";
import { z } from "../lib/validacion";
import { supabase, CLAVE_SESION } from "../lib/supabase";

interface Anfitrion {
  id: string;
  correo: string;
  nombre: string;
}
interface EstadoAuth {
  anfitrion: Anfitrion | null;
  cargando: boolean;
  verificado: boolean;
  error: string | null;
  iniciarSesion: () => Promise<void>;
  cerrarSesion: () => Promise<void>;
  verificar: () => void;
}
const Contexto = createContext<EstadoAuth | null>(null);
const filaAnfitrion = z.object({
  correo: z.email(),
  nombre: z.string().min(1),
});

export function AuthProvider({ children }: { children: ReactNode }) {
  const [anfitrion, setAnfitrion] = useState<Anfitrion | null>(null);
  const [cargando, setCargando] = useState(true);
  const [verificado, setVerificado] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const revision = useRef(0);
  const identidad = useRef<string | null>(null);
  const cierreSolicitado = useRef(false);
  const comprobar = useRef(() => {});

  useEffect(() => {
    let activo = true;
    let timer: ReturnType<typeof setTimeout> | undefined;

    function programar(sesion: Session | null) {
      if (cierreSolicitado.current && sesion) return;
      const turno = ++revision.current;
      clearTimeout(timer);
      const id = sesion?.user.id ?? null;
      if (identidad.current !== id) {
        identidad.current = id;
        setAnfitrion(null);
        setVerificado(false);
        setCargando(!!sesion);
      }
      if (!sesion) {
        setAnfitrion(null);
        setVerificado(false);
        setCargando(false);
        return;
      }
      // Salir del callback de Auth antes de consultar el mismo cliente evita
      // competir con el bloqueo interno que usa el refresco de tokens.
      timer = setTimeout(() => {
        void autorizar(sesion, turno);
      }, 0);
    }

    async function autorizar(sesion: Session, turno: number) {
      const actual = () => activo && revision.current === turno;
      try {
        const { data: usuario, error: errorUsuario } =
          await supabase.auth.getUser(sesion.access_token);
        if (!actual()) return;
        if (errorUsuario) throw errorUsuario;
        if (!usuario.user?.email || usuario.user.id !== sesion.user.id)
          throw new Error("Identidad no verificada");
        const { data, error: errorConsulta } = await supabase
          .from("anfitriones")
          .select("correo,nombre")
          .eq("correo", usuario.user.email)
          .maybeSingle();
        if (!actual()) return;
        if (errorConsulta) throw errorConsulta;
        if (!data) {
          setAnfitrion(null);
          setVerificado(false);
          setError(
            "Tu cuenta no está autorizada para agendar visitas. Contactá a administración para solicitar acceso.",
          );
          await supabase.auth.signOut({ scope: "local" });
          return;
        }
        const fila = filaAnfitrion.parse(data);
        if (fila.correo !== usuario.user.email)
          throw new Error("La autorización no corresponde a la cuenta");
        setAnfitrion({ id: usuario.user.id, ...fila });
        setVerificado(true);
        setError(null);
      } catch {
        if (!actual()) return;
        setVerificado(false);
        setError(
          "No pudimos verificar tu acceso. Revisá tu conexión y volvé a intentarlo.",
        );
      } finally {
        if (actual()) setCargando(false);
      }
    }

    async function recuperar() {
      const turno = revision.current;
      try {
        const { data, error: errorSesion } = await supabase.auth.getSession();
        if (!activo || turno !== revision.current) return;
        if (errorSesion) throw errorSesion;
        programar(data.session);
      } catch {
        if (!activo || turno !== revision.current) return;
        setError("No pudimos recuperar tu sesión. Volvé a iniciar sesión.");
        setVerificado(false);
        setCargando(false);
      }
    }

    const { data } = supabase.auth.onAuthStateChange((_evento, sesion) => {
      if (activo) programar(sesion);
    });
    comprobar.current = () => {
      void recuperar();
    };
    void recuperar();
    const alVolver = () => {
      if (document.visibilityState === "visible") void recuperar();
    };
    const alDesconectar = () => {
      ++revision.current;
      setVerificado(false);
      setError(
        "Estás sin conexión. Tus cambios siguen en esta pestaña; reconectá para continuar.",
      );
    };
    const intervalo = setInterval(alVolver, 60_000);
    document.addEventListener("visibilitychange", alVolver);
    window.addEventListener("online", alVolver);
    window.addEventListener("offline", alDesconectar);
    return () => {
      activo = false;
      ++revision.current;
      clearTimeout(timer);
      clearInterval(intervalo);
      data.subscription.unsubscribe();
      document.removeEventListener("visibilitychange", alVolver);
      window.removeEventListener("online", alVolver);
      window.removeEventListener("offline", alDesconectar);
    };
  }, []);

  const iniciarSesion = useCallback(async () => {
    cierreSolicitado.current = false;
    setError(null);
    try {
      const { error: fallo } = await supabase.auth.signInWithOAuth({
        provider: "google",
        options: {
          redirectTo: `${window.location.origin}/auth/callback`,
          queryParams: { prompt: "select_account" },
        },
      });
      if (fallo) throw fallo;
    } catch {
      setError(
        "No pudimos abrir el inicio de sesión con Google. Intentá de nuevo.",
      );
    }
  }, []);

  const cerrarSesion = useCallback(async () => {
    cierreSolicitado.current = true;
    ++revision.current;
    identidad.current = null;
    setAnfitrion(null);
    setVerificado(false);
    setCargando(false);
    try {
      const { error: fallo } = await supabase.auth.signOut({ scope: "local" });
      if (fallo) throw fallo;
    } catch {
      setError(
        "La sesión se cerró en esta pestaña, pero no pudimos confirmar el cierre remoto.",
      );
    } finally {
      for (const clave of [
        CLAVE_SESION,
        `${CLAVE_SESION}-code-verifier`,
        `${CLAVE_SESION}-user`,
      ]) {
        try {
          sessionStorage.removeItem(clave);
        } catch {
          /* El navegador puede bloquear el almacenamiento. */
        }
      }
    }
    if (window.location.hostname === "visitas.megabrisas.com") {
      window.location.assign("/cdn-cgi/access/logout");
    }
  }, []);

  return (
    <Contexto.Provider
      value={{
        anfitrion,
        cargando,
        verificado,
        error,
        iniciarSesion,
        cerrarSesion,
        verificar: () => comprobar.current(),
      }}
    >
      {children}
    </Contexto.Provider>
  );
}

export function useAuth() {
  const estado = useContext(Contexto);
  if (!estado) throw new Error("Falta el proveedor de sesión.");
  return estado;
}
