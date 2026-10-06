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
import { normalizarCorreo } from "../lib/contrasena";
import { supabase, CLAVE_SESION } from "../lib/supabase";

interface Anfitrion {
  id: string;
  correo: string;
  nombre: string;
}
/** Resultado de una acción de la pantalla de ingreso: el mensaje es para el
 * usuario y nunca dice si una cuenta existe (OWASP: enumeración). */
export type Resultado = { ok: true } | { ok: false; mensaje: string };
interface EstadoAuth {
  anfitrion: Anfitrion | null;
  cargando: boolean;
  verificado: boolean;
  error: string | null;
  iniciarSesion: (correo: string, contrasena: string) => Promise<Resultado>;
  /** Primer ingreso o después de un "restablecer" del panel: con el código
   * de activación define su contraseña y entra. */
  activarCuenta: (correo: string, codigo: string, contrasena: string) => Promise<Resultado>;
  cerrarSesion: () => Promise<void>;
  verificar: () => void;
}
const Contexto = createContext<EstadoAuth | null>(null);
const filaAnfitrion = z.object({
  correo: z.email(),
  nombre: z.string().min(1),
});

const SIN_CONEXION =
  "No se pudo comunicar con el servidor. Revise su conexión e intente de nuevo.";
const DEMASIADOS_INTENTOS =
  "Demasiados intentos. Espere unos minutos antes de volver a intentar.";

/** Código y estado HTTP de un error de Supabase Auth, sin depender de su
 * clase (un fallo de red no trae ninguno de los dos). */
function detalle(fallo: unknown): { codigo?: string; estado?: number } {
  if (typeof fallo !== "object" || fallo === null) return {};
  const { code, status } = fallo as { code?: unknown; status?: unknown };
  return {
    codigo: typeof code === "string" ? code : undefined,
    estado: typeof status === "number" ? status : undefined,
  };
}
function esLimite(fallo: unknown) {
  const { codigo, estado } = detalle(fallo);
  return estado === 429 || (codigo?.startsWith("over_") ?? false);
}

/** `{ error, detail }` de una Edge Function que respondió con error. */
async function cuerpoDeError(fallo: unknown): Promise<{ error?: string; detail?: string } | null> {
  const contexto = (fallo as { context?: unknown } | null)?.context;
  if (!(contexto instanceof Response)) return null;
  try {
    const cuerpo: unknown = await contexto.clone().json();
    return typeof cuerpo === "object" && cuerpo !== null ? cuerpo : null;
  } catch {
    return null;
  }
}

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
            "Su cuenta no está autorizada para agendar visitas. Pida acceso a administración.",
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
          "No se pudo verificar su acceso. Revise su conexión e intente de nuevo.",
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
        setError("No se pudo recuperar su sesión. Vuelva a iniciar sesión.");
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
        "Sin conexión. Sus cambios siguen en esta pestaña; cuando vuelva la conexión puede continuar.",
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

  const iniciarSesion = useCallback(
    async (correo: string, contrasena: string): Promise<Resultado> => {
      cierreSolicitado.current = false;
      setError(null);
      try {
        const { error: fallo } = await supabase.auth.signInWithPassword({
          email: normalizarCorreo(correo),
          password: contrasena,
        });
        if (!fallo) return { ok: true };
        if (esLimite(fallo)) return { ok: false, mensaje: DEMASIADOS_INTENTOS };
        const { estado } = detalle(fallo);
        // 400/401/403: credenciales, correo sin confirmar o cuenta
        // bloqueada. El mismo mensaje para todo: no revela si la cuenta
        // existe ni en qué estado está.
        if (estado !== undefined && estado >= 400 && estado < 500)
          return { ok: false, mensaje: "Correo o contraseña incorrectos." };
        return { ok: false, mensaje: SIN_CONEXION };
      } catch {
        return { ok: false, mensaje: SIN_CONEXION };
      }
    },
    [],
  );

  const activarCuenta = useCallback(
    async (correo: string, codigo: string, contrasena: string): Promise<Resultado> => {
      try {
        const { error: fallo } = await supabase.functions.invoke("anfitrion-activar", {
          body: { correo: normalizarCorreo(correo), codigo, contrasena },
        });
        if (fallo) {
          const cuerpo = await cuerpoDeError(fallo);
          // `detail` es para la persona (mismo texto para todo código
          // rechazado: no revela si la cuenta existe).
          if (cuerpo?.detail && cuerpo.error !== "error")
            return { ok: false, mensaje: cuerpo.detail };
          return { ok: false, mensaje: SIN_CONEXION };
        }
      } catch {
        return { ok: false, mensaje: SIN_CONEXION };
      }
      // Activada: entra con la contraseña que acaba de elegir.
      const ingreso = await iniciarSesion(correo, contrasena);
      return ingreso.ok
        ? ingreso
        : {
            ok: false,
            mensaje: "Su cuenta quedó activada, pero no se pudo ingresar. Ingrese con su correo y la contraseña nueva.",
          };
    },
    [iniciarSesion],
  );

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
        "La sesión se cerró en esta pestaña, pero no se pudo confirmar el cierre en el servidor.",
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
        activarCuenta,
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
