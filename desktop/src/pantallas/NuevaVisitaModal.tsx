import { useState } from "react";
import { CalendarCheck, Mail } from "lucide-react";
import Modal from "../componentes/Modal";
import SegmentadoOpciones from "../componentes/SegmentadoOpciones";
import type { OpcionSegmentada } from "../componentes/SegmentadoOpciones";
import { CheckInAgendada } from "./VisitaCheckInModal";
import { FormularioPorCorreo } from "./FormularioPorCorreo";

type Modo = "agendada" | "correo";

const OPCIONES_MODO: OpcionSegmentada<Modo>[] = [
  { valor: "agendada", Icono: CalendarCheck, titulo: "Agendada en la web" },
  { valor: "correo", Icono: Mail, titulo: "Autorizada por correo" },
];

const TEXTO_MODO: Record<Modo, string> = {
  agendada: "Agendada en la web",
  correo: "Autorizada por correo",
};

/**
 * Un solo modal para registrar la entrada de una visita (pedido del dueño
 * 2026-10-05), con el mismo interruptor de íconos que el modal de salida:
 * - "Agendada" (camino principal): busca la cita por cédula.
 * - "Por correo": el guarda la registra a mano, respaldada por un correo.
 * Si la cédula no tiene cita para hoy y nada lo impide, "Agendada" ofrece
 * pasar a "Por correo" con la cédula ya escrita.
 */
export default function NuevaVisitaModal({
  cedulaInicial,
  onRegistrado,
  onCerrar,
}: {
  /** Desde "Esperadas": la cédula de la visita elegida. */
  cedulaInicial?: string;
  onRegistrado: () => void;
  onCerrar: () => void;
}) {
  const [modo, setModo] = useState<Modo>("agendada");
  const [cedulaCorreo, setCedulaCorreo] = useState("");

  return (
    <Modal titulo="Nueva visita" onCerrar={onCerrar}>
      <div style={{ display: "flex", flexDirection: "column", gap: "0.9rem" }}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: "0.75rem" }}>
          <span style={{ fontWeight: 600, color: "var(--texto)" }}>{TEXTO_MODO[modo]}</span>
          <SegmentadoOpciones
            opciones={OPCIONES_MODO}
            valor={modo}
            onCambiar={(nuevo) => {
              setCedulaCorreo("");
              setModo(nuevo);
            }}
            etiqueta="Tipo de visita"
          />
        </div>
        {modo === "agendada" ? (
          <CheckInAgendada
            cedulaInicial={cedulaInicial}
            onRegistrado={onRegistrado}
            onPorCorreo={(cedula) => {
              setCedulaCorreo(cedula);
              setModo("correo");
            }}
          />
        ) : (
          <FormularioPorCorreo key={cedulaCorreo} cedulaInicial={cedulaCorreo} onRegistrado={onRegistrado} />
        )}
      </div>
    </Modal>
  );
}
