import { sumarDias } from "./calculo";
import { ESTADOS_PRAIND, TIPOS_INGRESO } from "./catalogo";
import type { Filtro, Lienzo } from "./esquema";

/**
 * Traduce los filtros de un lienzo ya validado a los parámetros de las
 * funciones SQL `lienzo_contratistas` y `lienzo_ingresos` (migración
 * `lienzo_catalogo`). Cada parámetro admite un solo filtro: uno repetido sobre
 * el mismo campo se rechaza en vez de combinarse a escondidas.
 */
export interface ParametrosContratistas {
  p_empresas?: string[];
  p_empresas_excluir?: string[];
  p_contiene_empresa?: string;
  p_estados_praind?: string[];
  p_tipos_ingreso?: string[];
  p_sitio?: string;
  p_acceso_activo?: boolean;
  p_vencimiento_desde?: string;
  p_vencimiento_hasta?: string;
  p_contiene_nombre?: string;
  p_dias_desde?: number;
  p_dias_hasta?: number;
}

export interface ParametrosIngresos {
  p_desde: string;
  p_hasta: string;
  p_empresas?: string[];
  p_empresas_excluir?: string[];
  p_contiene_empresa?: string;
  p_tipos_ingreso?: string[];
  p_sitio?: string;
  p_contiene_nombre?: string;
}

type Parametros = Record<string, unknown>;

function lista(filtro: Filtro): string[] {
  return (Array.isArray(filtro.valor) ? filtro.valor : [filtro.valor]).map(String);
}

function asignar(parametros: Parametros, clave: string, valor: unknown, filtro: Filtro) {
  if (clave in parametros) throw new Error(`Hay dos filtros sobre "${filtro.campo}" que se pisan`);
  parametros[clave] = valor;
}

/** "no_en" sobre una enumeración = todos los valores del catálogo menos esos. */
function valoresDeEnumeracion(filtro: Filtro, catalogo: readonly string[]): string[] {
  const dados = lista(filtro);
  return filtro.op === "no_en" ? catalogo.filter((v) => !dados.includes(v)) : dados;
}

function rango(filtro: Filtro): [unknown, unknown] {
  if (filtro.op === "entre" && Array.isArray(filtro.valor)) return [filtro.valor[0], filtro.valor[1]];
  return [filtro.valor, filtro.valor];
}

function empresa(parametros: Parametros, filtro: Filtro) {
  if (filtro.op === "contiene") asignar(parametros, "p_contiene_empresa", String(filtro.valor), filtro);
  else if (filtro.op === "no_en") asignar(parametros, "p_empresas_excluir", lista(filtro), filtro);
  else asignar(parametros, "p_empresas", lista(filtro), filtro);
}

export function parametrosContratistas(lienzo: Lienzo): ParametrosContratistas {
  const p: Parametros = {};
  for (const filtro of lienzo.filtros) {
    switch (filtro.campo) {
      case "nombre":
        asignar(p, "p_contiene_nombre", String(filtro.valor), filtro);
        break;
      case "empresa":
        empresa(p, filtro);
        break;
      case "estado_praind":
        asignar(p, "p_estados_praind", valoresDeEnumeracion(filtro, ESTADOS_PRAIND), filtro);
        break;
      case "tipo_ingreso":
        asignar(p, "p_tipos_ingreso", valoresDeEnumeracion(filtro, TIPOS_INGRESO), filtro);
        break;
      case "sitio":
        asignar(p, "p_sitio", String(filtro.valor), filtro);
        break;
      case "acceso_activo":
        asignar(p, "p_acceso_activo", filtro.valor, filtro);
        break;
      case "vencimiento": {
        const [desde, hasta] = rango(filtro);
        asignar(p, "p_vencimiento_desde", desde, filtro);
        asignar(p, "p_vencimiento_hasta", hasta, filtro);
        break;
      }
      case "dias_para_vencer": {
        const [desde, hasta] = rango(filtro);
        asignar(p, "p_dias_desde", desde, filtro);
        asignar(p, "p_dias_hasta", hasta, filtro);
        break;
      }
      default:
        throw new Error(`Campo "${filtro.campo}" sin consulta en contratistas`);
    }
  }
  return p as ParametrosContratistas;
}

/** `hoy` (YYYY-MM-DD, día de Costa Rica) fija el rango por omisión: las últimas 2 semanas. */
export function parametrosIngresos(lienzo: Lienzo, hoy: string): ParametrosIngresos {
  const p: Parametros = {};
  for (const filtro of lienzo.filtros) {
    switch (filtro.campo) {
      case "nombre":
        asignar(p, "p_contiene_nombre", String(filtro.valor), filtro);
        break;
      case "empresa":
        empresa(p, filtro);
        break;
      case "tipo_ingreso":
        asignar(p, "p_tipos_ingreso", valoresDeEnumeracion(filtro, TIPOS_INGRESO), filtro);
        break;
      case "sitio":
        asignar(p, "p_sitio", String(filtro.valor), filtro);
        break;
      case "dia": {
        const [desde, hasta] = rango(filtro);
        asignar(p, "p_desde", desde, filtro);
        asignar(p, "p_hasta", hasta, filtro);
        break;
      }
      default:
        throw new Error(`Campo "${filtro.campo}" sin consulta en ingresos`);
    }
  }
  p.p_hasta ??= hoy;
  p.p_desde ??= sumarDias(p.p_hasta as string, -13);
  return p as unknown as ParametrosIngresos;
}
