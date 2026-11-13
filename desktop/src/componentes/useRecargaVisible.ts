import { useEffect, useState } from "react";
import { useSeccionActiva } from "../contexto/BarraEstadoContexto";
import {
  EVENTO_CAMBIO_EN_VIVO,
  EVENTO_CAMBIO_LOCAL_NUBE,
  EVENTO_NUBE_ACTUALIZADA,
} from "../eventosNube";

/** Eventos que dejan desactualizada una pantalla que no se está viendo:
 * una sincronización, un aviso en vivo, o un registro hecho en esta misma PC
 * desde otra pantalla (`solicitarSincronizacionNube`). */
const EVENTOS_DE_CAMBIO = [EVENTO_NUBE_ACTUALIZADA, EVENTO_CAMBIO_EN_VIVO, EVENTO_CAMBIO_LOCAL_NUBE];

/**
 * Versión de los datos que la pantalla debe mostrar: sube cuando hay que
 * recargar, pero SÓLO mientras la pantalla está a la vista.
 *
 * La app deja montada cada sección visitada (ver `visitadas` en App.tsx), y
 * antes todas recargaban en cada sincronización, aviso en vivo o pulso
 * periódico aunque estuvieran ocultas -- con varias pantallas abiertas eran
 * decenas de consultas cada pocos minutos, incluidos historiales de 6 meses
 * que nadie estaba mirando (auditoría 2026-11-13). Ahora:
 *
 * - visible: `senal` cambia -> la versión sube enseguida;
 * - oculta: lo que llegue (`senal` o un evento de cambio) sólo la marca como
 *   pendiente, y al volver a mostrarse sube UNA vez.
 *
 * `senal` es el contador de App (`refrescarActivos`) para las pantallas que
 * lo reciben; las que escuchan los eventos por su cuenta (`useCargaAlCambiar`)
 * no lo pasan. Usar la versión como dependencia del efecto que carga.
 */
export function useRecargaVisible(senal = 0): number {
  const activa = useSeccionActiva();
  const [estado, setEstado] = useState({ senal, version: 0, pendiente: false });

  // Ajuste durante el render (patrón de React, sin efecto): al estar a la
  // vista con algo pendiente o una señal nueva, se sube la versión.
  if (activa && (estado.pendiente || estado.senal !== senal)) {
    setEstado({ senal, version: estado.version + 1, pendiente: false });
  }

  useEffect(() => {
    if (activa) return;
    const marcar = () => setEstado((actual) => (actual.pendiente ? actual : { ...actual, pendiente: true }));
    for (const evento of EVENTOS_DE_CAMBIO) window.addEventListener(evento, marcar);
    return () => {
      for (const evento of EVENTOS_DE_CAMBIO) window.removeEventListener(evento, marcar);
    };
  }, [activa]);

  return estado.version;
}
