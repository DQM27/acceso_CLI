import { createClient } from "@supabase/supabase-js";

export const CLAVE_SESION = "brisas-visitas-auth";

// La sesión dura lo que esta pestaña: no se comparten tokens con el panel ni
// quedan datos de visitantes en el equipo. El ingreso es con correo y
// contraseña; el primer ingreso, con el código de activación que entrega
// administración (Edge Function `anfitrion-activar`). Nada pasa por la URL.
//
// URL/clave vienen de `.env` (versionado, valores de producción por
// defecto) -- para apuntar el build local a staging sin tocar ese
// archivo, crear un `.env.local` (gitignored) con
// VITE_SUPABASE_URL/VITE_SUPABASE_PUBLISHABLE_KEY y los valores de
// `docs/recuperacion-sitio-staging.md`.
export const supabase = createClient(
  import.meta.env.VITE_SUPABASE_URL,
  import.meta.env.VITE_SUPABASE_PUBLISHABLE_KEY,
  {
    auth: {
      flowType: "pkce",
      storageKey: CLAVE_SESION,
      storage: {
        getItem: (clave) => {
          try {
            return window.sessionStorage.getItem(clave);
          } catch {
            return null;
          }
        },
        setItem: (clave, valor) => window.sessionStorage.setItem(clave, valor),
        removeItem: (clave) => {
          try {
            window.sessionStorage.removeItem(clave);
          } catch {
            /* Puede estar bloqueado por el navegador. */
          }
        },
      },
      persistSession: true,
      autoRefreshToken: true,
      // Sin OAuth ni enlaces de correo no hay retorno que leer de la URL: nada
      // de lo que llegue en la dirección abre una sesión por sí solo.
      detectSessionInUrl: false,
    },
    global: {
      fetch: (entrada, opciones) =>
        fetch(entrada, {
          ...opciones,
          cache: "no-store",
          signal: opciones?.signal
            ? AbortSignal.any([opciones.signal, AbortSignal.timeout(20_000)])
            : AbortSignal.timeout(20_000),
        }),
    },
  },
);
