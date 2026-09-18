import { createClient } from "@supabase/supabase-js";

export const CLAVE_SESION = "brisas-visitas-auth";

// La sesión dura lo que esta pestaña. El verificador PKCE sobrevive al retorno
// de Google sin compartir tokens con el panel ni guardar datos de visitantes.
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
      detectSessionInUrl: true,
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
