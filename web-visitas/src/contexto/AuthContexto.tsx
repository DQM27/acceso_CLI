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
  /** Entró con un código de correo y todavía no definió su contraseña: la
   * agenda espera hasta que lo haga. */
  debeDefinirContrasena: boolean;
  iniciarSesion: (correo: string, contrasena: string) => Promise<Resultado>;
  solicitarCodigo: (correo: string) => Promise<Resultado>;
  verificarCodigo: (correo: string, codigo: string) => Promise<Resultado>;
  definirContrasena: (contrasena: string) => Promise<Resultado>;
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

// Marca "falta definir la contraseña" en la pestaña: si recarga a mitad del
// paso, vuelve a la misma pantalla en vez de entrar a la agenda.
const CLAVE_DEFINIR = `${CLAVE_SESION}-definir-contrasena`;
function leerMarcaDefinir(): boolean {
  try {
    return sessionStorage.getItem(CLAVE_DEFINIR) === "1";
  } catch {
    return false;
  }
}
function guardarMarcaDefinir(valor: boolean) {
  try {
    if (valor) sessionStorage.setItem(CLAVE_DEFINIR, "1");
    else sessionStorage.removeItem(CLAVE_DEFINIR);
  } catch {
    /* Sin almacenamiento la marca vive solo en memoria. */
  }
}

export function AuthProvider({ children }: { children: ReactNode }) {
  const [anfitrion, setAnfitrion] = useState<Anfitrion | null>(null);
  const [cargando, setCargando] = useState(true);
  const [verificado, setVerificado] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [debeDefinirContrasena, setDebeDefinir] = useState(leerMarcaDefinir);
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
        guardarMarcaDefinir(false);
        setDebeDefinir(false);
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

  const solicitarCodigo = useCallback(
    async (correo: string): Promise<Resultado> => {
      try {
        // `shouldCreateUser: false`: las cuentas se dan de alta por SQL
        // (tabla `anfitriones`), nunca desde acá. La plantilla "Magic Link"
        // del proyecto manda el código (`{{ .Token }}`), no un enlace.
        const { error: fallo } = await supabase.auth.signInWithOtp({
          email: normalizarCorreo(correo),
          options: { shouldCreateUser: false },
        });
        if (!fallo) return { ok: true };
        if (esLimite(fallo)) return { ok: false, mensaje: DEMASIADOS_INTENTOS };
        const { estado } = detalle(fallo);
        // Un correo sin cuenta responde 422 ("Signups not allowed for otp")
        // y uno bloqueado, otro 4xx. Se contesta igual que un envío exitoso
        // para no revelar quién es anfitrión.
        if (estado !== undefined && estado >= 400 && estado < 500)
          return { ok: true };
        return { ok: false, mensaje: SIN_CONEXION };
      } catch {
        return { ok: false, mensaje: SIN_CONEXION };
      }
    },
    [],
  );

  const verificarCodigo = useCallback(
    async (correo: string, codigo: string): Promise<Resultado> => {
      cierreSolicitado.current = false;
      setError(null);
      try {
        const { error: fallo } = await supabase.auth.verifyOtp({
          email: normalizarCorreo(correo),
          token: codigo.replace(/\s+/g, ""),
          type: "email",
        });
        if (!fallo) {
          guardarMarcaDefinir(true);
          setDebeDefinir(true);
          return { ok: true };
        }
        if (esLimite(fallo)) return { ok: false, mensaje: DEMASIADOS_INTENTOS };
        const { estado } = detalle(fallo);
        if (estado !== undefined && estado >= 400 && estado < 500)
          return {
            ok: false,
            mensaje: "Código incorrecto o vencido. Revise el último correo o pida otro código.",
          };
        return { ok: false, mensaje: SIN_CONEXION };
      } catch {
        return { ok: false, mensaje: SIN_CONEXION };
      }
    },
    [],
  );

  const definirContrasena = useCallback(
    async (contrasena: string): Promise<Resultado> => {
      try {
        const { error: fallo } = await supabase.auth.updateUser({
          password: contrasena,
        });
        if (!fallo) {
          guardarMarcaDefinir(false);
          setDebeDefinir(false);
          return { ok: true };
        }
        if (esLimite(fallo)) return { ok: false, mensaje: DEMASIADOS_INTENTOS };
        const { codigo, estado } = detalle(fallo);
        if (codigo === "weak_password")
          return {
            ok: false,
            mensaje:
              "El servidor rechazó esa contraseña por débil o porque aparece en filtraciones conocidas. Use otra frase.",
          };
        if (codigo === "same_password")
          return { ok: false, mensaje: "Use una contraseña distinta de la anterior." };
        if (estado === 401 || codigo === "session_not_found" || codigo === "session_expired")
          return {
            ok: false,
            mensaje:
              "La sesión del código terminó. Pida un código nuevo desde la pantalla de ingreso.",
          };
        return { ok: false, mensaje: SIN_CONEXION };
      } catch {
        return { ok: false, mensaje: SIN_CONEXION };
      }
    },
    [],
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
      guardarMarcaDefinir(false);
      setDebeDefinir(false);
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
        debeDefinirContrasena,
        solicitarCodigo,
        verificarCodigo,
        definirContrasena,
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
