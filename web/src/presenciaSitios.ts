import { useEffect, useState } from "react";
import { supabase } from "./lib/supabase";
import type { RealtimeChannel } from "@supabase/supabase-js";

/**
 * Un solo canal Realtime por sitio, compartido entre quien lo necesite --
 * ver docs/features-futuras/plan-sesion-unica-dispositivos.md, "Panel de presencia en tiempo
 * real". Antes Dispositivos.tsx y Usuarios.tsx abrían cada uno su propia
 * suscripción a `sitio:{id}` por separado; como las secciones del panel
 * quedan montadas de fondo una vez visitadas (ver el comentario en
 * App.tsx sobre `visitadas`), las dos terminaban vivas a la vez y
 * supabase-js tira "cannot add `presence` callbacks... after `subscribe()`"
 * cuando la segunda intenta registrar su propio handler sobre lo que cree
 * que es un canal nuevo. Acá se cuenta cuántos consumidores hay por sitio
 * (`listeners`) y sólo se abre/cierra el canal real cuando el primero
 * llega o el último se va.
 */
type EstadoPresencia = Record<string, unknown[]>;
type Escucha = (estado: EstadoPresencia) => void;

interface Entrada {
  canal: RealtimeChannel;
  escuchas: Set<Escucha>;
  ultimoEstado: EstadoPresencia;
}

const entradasPorSitio = new Map<string, Entrada>();

function suscribirse(sitioId: string, escucha: Escucha): () => void {
  let entrada = entradasPorSitio.get(sitioId);
  if (!entrada) {
    const canal = supabase.channel(`sitio:${sitioId}`, { config: { private: true } });
    const nueva: Entrada = { canal, escuchas: new Set(), ultimoEstado: {} };
    canal
      .on("presence", { event: "sync" }, () => {
        nueva.ultimoEstado = canal.presenceState();
        nueva.escuchas.forEach((fn) => fn(nueva.ultimoEstado));
      })
      .subscribe();
    entradasPorSitio.set(sitioId, nueva);
    entrada = nueva;
  }
  entrada.escuchas.add(escucha);
  escucha(entrada.ultimoEstado);
  return () => {
    const actual = entradasPorSitio.get(sitioId);
    if (!actual) return;
    actual.escuchas.delete(escucha);
    if (actual.escuchas.size === 0) {
      void supabase.removeChannel(actual.canal);
      entradasPorSitio.delete(sitioId);
    }
  };
}

/** Estado de presencia crudo por sitio -- cada consumidor decide qué campo
 * del payload le importa (`dispositivo_id` en Dispositivos.tsx,
 * `usuario_cedula` en Usuarios.tsx). */
export function usePresenciaPorSitio(sitioIds: string[]): Record<string, EstadoPresencia> {
  const [estadoPorSitio, setEstadoPorSitio] = useState<Record<string, EstadoPresencia>>({});

  useEffect(() => {
    if (sitioIds.length === 0) return;
    const cancelaciones = sitioIds.map((sitioId) =>
      suscribirse(sitioId, (estado) => {
        setEstadoPorSitio((actual) => ({ ...actual, [sitioId]: estado }));
      }),
    );
    return () => {
      cancelaciones.forEach((cancelar) => cancelar());
      setEstadoPorSitio({});
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- sitioIds es un array nuevo en cada render; comparar por contenido evita resuscribirse en cada recarga de `dispositivos`/`usuarios`.
  }, [sitioIds.join(",")]);

  return estadoPorSitio;
}
