import { createClient } from "@supabase/supabase-js";

export const CLAVE_SESION = "brisas-visitas-auth";

// La sesión dura lo que esta pestaña. El verificador PKCE sobrevive al retorno
// de Google sin compartir tokens con el panel ni guardar datos de visitantes.
export const supabase = createClient(
  "https://xidaepyaljzkpbsxrqsm.supabase.co",
  "sb_publishable_Sr9DPGMD7MFirLQfG7ViWg_6pJeEpqU",
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
