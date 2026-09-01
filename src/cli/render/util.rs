//! Formateo de texto compartido por varias Surfaces (búsqueda, activos,
//! historial, formularios): traducir enums de dominio a texto, horas
//! relativas y recorte seguro de UTF-8.

use chrono::{DateTime, Utc};

use crate::models::medio_ingreso::MedioIngreso;
use crate::models::tipo_ingreso::TipoIngreso;
use crate::models::usuario::RolUsuario;
use crate::tiempo::a_costa_rica;

pub(super) fn tipo_texto(tipo: TipoIngreso) -> &'static str {
    match tipo {
        TipoIngreso::Praind => "PRAIND",
        TipoIngreso::InHouse => "IN HOUSE",
        TipoIngreso::PorCorreo => "POR CORREO",
        TipoIngreso::Swat => "SWAT",
    }
}

pub(super) fn medio_texto(medio: MedioIngreso) -> &'static str {
    match medio {
        MedioIngreso::Caminando => "CAMINANDO",
        MedioIngreso::Vehiculo => "VEHICULO",
    }
}

pub(super) fn rol_texto(rol: RolUsuario) -> &'static str {
    match rol {
        RolUsuario::Root => "ROOT",
        RolUsuario::Administrador => "ADMINISTRADOR",
        RolUsuario::Operador => "OPERADOR",
    }
}

pub(super) fn si_no(valor: bool) -> &'static str {
    if valor { "Sí" } else { "No" }
}

pub(super) fn hora_cr(instante: DateTime<Utc>) -> String {
    a_costa_rica(instante).format("%H:%M").to_string()
}

/// "2 h 15 min" / "45 min" — duración desde `desde` hasta ahora. Un instante
/// futuro (reloj inconsistente) se reporta como "0 min", nunca negativo.
pub(super) fn duracion_texto(desde: DateTime<Utc>) -> String {
    let minutos = (Utc::now() - desde).num_minutes().max(0);
    let horas = minutos / 60;
    if horas > 0 {
        format!("{horas} h {:02} min", minutos % 60)
    } else {
        format!("{minutos} min")
    }
}

pub(super) fn cantidad_personas(total: usize) -> String {
    if total == 1 {
        "1 persona".to_string()
    } else {
        format!("{total} personas")
    }
}

/// Trunca a `ancho` columnas añadiendo "…" cuando recorta — por caracteres,
/// no por bytes, para no romper UTF-8.
pub(super) fn recortar(texto: &str, ancho: usize) -> String {
    if texto.chars().count() <= ancho {
        return texto.to_string();
    }
    let mut recortado: String = texto.chars().take(ancho.saturating_sub(1)).collect();
    recortado.push('…');
    recortado
}
