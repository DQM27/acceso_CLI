import { invoke } from "@tauri-apps/api/core";
import type { ContratistaResumen } from "./contratistas";
import type { EmpresaResumen } from "./empresas";
import type { UsuarioResumen } from "./usuarios";
import type { IngresoActivoResumen, MedioIngreso, PreparacionIngreso } from "./ingresos";
import type { CambioAuditado } from "./auditoria";

// Espejo de comandos/consola.rs (GUI) y src/comandos/estado.rs::ContextState
// (núcleo) — piloto de la consola tipo terminal. Reusa el mismo parser +
// resolver que `--comandos`; ver `src/application/comandos.rs`.
//
// `CambioAuditado` se reusa de `api/auditoria.ts` (mismo tipo del núcleo, ya
// no es sólo el piloto que no leía sus campos — ahora también lo usa la
// pantalla Auditoría real).

export type ContextState =
  | { Inicio: { total_dentro: number } }
  | {
      Coincidencias: {
        consulta: string;
        items: ContratistaResumen[];
        seleccion: number;
        offset: number;
        total: number;
      };
    }
  | {
      CoincidenciasEmpresas: {
        consulta: string;
        items: EmpresaResumen[];
        seleccion: number;
        offset: number;
        hay_mas: boolean;
      };
    }
  | {
      CoincidenciasUsuarios: {
        consulta: string;
        items: UsuarioResumen[];
        seleccion: number;
        offset: number;
        hay_mas: boolean;
      };
    }
  | {
      CoincidenciasActivos: {
        descripcion: string;
        items: IngresoActivoResumen[];
        seleccion: number;
      };
    }
  | {
      ResumenIngreso: {
        preparacion: PreparacionIngreso;
        gafete: number | null;
        medio: MedioIngreso;
        gafete_ocupante: IngresoActivoResumen | null;
      };
    }
  | { ResumenSalida: { activo: IngresoActivoResumen } }
  | { TablaActivos: { items: IngresoActivoResumen[]; total: number; seleccion: number } }
  | {
      TablaAuditoria: {
        items: CambioAuditado[];
        seleccion: number;
        offset: number;
        total: number;
      };
    }
  | { FichaContratista: { resumen: ContratistaResumen } }
  | "ConfirmarCerrarSesion"
  | "ConfirmarCambioPassword"
  | "ConfirmarModoClasico"
  | "NuevoContratista"
  | "NuevoEmpresa"
  | "NuevoUsuario"
  | "AbrirHistorial"
  | { AbrirSalidaGafete: { texto: string } }
  | "Ayuda"
  | { MensajeError: { mensaje: string } };

export function ejecutarComando(texto: string): Promise<ContextState> {
  return invoke("ejecutar_comando", { texto });
}

export interface Autocompletado {
  sugerencias: string[];
  completado: string | null;
}

export function autocompletarComando(texto: string): Promise<Autocompletado> {
  return invoke("autocompletar_comando", { texto });
}
