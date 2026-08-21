//! Helpers del formulario de alta/edición de Contratistas — validación,
//! construcción de los datos a guardar y navegación entre campos.

use chrono::NaiveDate;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{
    models::tipo_ingreso::TipoIngreso,
    services::contratista_service::{DatosActualizacionContratista, DatosContratista},
    tui::ui_kit::TextInput,
};

use super::{CampoFormulario, FormularioContratista};

pub(in crate::tui::contratistas::state) fn tipos() -> [TipoIngreso; 4] {
    [
        TipoIngreso::Praind,
        TipoIngreso::InHouse,
        TipoIngreso::PorCorreo,
        TipoIngreso::Swat,
    ]
}

pub(in crate::tui::contratistas::state) fn texto_tipo(t: TipoIngreso) -> &'static str {
    match t {
        TipoIngreso::Praind => "PRAIND",
        TipoIngreso::InHouse => "IN HOUSE",
        TipoIngreso::PorCorreo => "POR CORREO",
        TipoIngreso::Swat => "SWAT",
    }
}

pub(super) fn construir(
    f: &FormularioContratista,
    empresa_id: Option<i64>,
) -> Result<DatosContratista, String> {
    if f.cedula.value().trim().is_empty() {
        return Err("La cédula es obligatoria".into());
    }
    if f.nombre.value().trim().is_empty() {
        return Err("El nombre es obligatorio".into());
    }
    let fecha = if f.requiere_praind() {
        Some(
            NaiveDate::parse_from_str(f.fecha_praind.value(), "%d/%m/%Y").map_err(|_| {
                if f.fecha_praind.value().is_empty() {
                    "Fecha PRAIND requerida"
                } else {
                    "Fecha inválida. Use DD/MM/YYYY"
                }
            })?,
        )
    } else {
        None
    };
    Ok(DatosContratista {
        cedula: f.cedula.value().trim().into(),
        nombre: f.nombre.value().trim().into(),
        empresa_id: empresa_id.ok_or("La empresa seleccionada ya no existe")?,
        tipo_ingreso: f.tipo,
        fecha_vencimiento_praind: fecha,
        es_personal_ruta: f.personal_ruta,
        tiene_acceso: f.tiene_acceso,
    })
}

pub(super) fn convertir_actualizacion(datos: DatosContratista) -> DatosActualizacionContratista {
    DatosActualizacionContratista {
        cedula: datos.cedula,
        nombre: datos.nombre,
        empresa_id: datos.empresa_id,
        tipo_ingreso: datos.tipo_ingreso,
        fecha_vencimiento_praind: datos.fecha_vencimiento_praind,
        es_personal_ruta: datos.es_personal_ruta,
        tiene_acceso: datos.tiene_acceso,
    }
}

pub(super) fn mover_campo(f: &mut FormularioContratista, d: isize) {
    let habilitados: Vec<usize> = CampoFormulario::TODOS
        .iter()
        .enumerate()
        .filter_map(|(indice, campo)| {
            let habilitado = (*campo != CampoFormulario::Cedula || f.cedula_editable)
                && (*campo != CampoFormulario::FechaPraind || f.requiere_praind());
            habilitado.then_some(indice)
        })
        .collect();
    let posicion = habilitados
        .iter()
        .position(|indice| *indice == f.campo)
        .unwrap_or(0);
    let len = habilitados.len() as isize;
    let siguiente = (posicion as isize + d).rem_euclid(len) as usize;
    f.campo = habilitados[siguiente];
}

pub(super) fn agregar_fecha(input: &mut TextInput, key: KeyEvent, caracter: char) {
    if caracter == '/' {
        input.handle_key(key);
        return;
    }
    if !caracter.is_ascii_digit() {
        return;
    }
    let cantidad_digitos = input.value().chars().filter(char::is_ascii_digit).count();
    if cantidad_digitos < 8 {
        let cursor_al_final = input.cursor() == input.value().chars().count();
        if cursor_al_final && matches!(cantidad_digitos, 2 | 4) && !input.value().ends_with('/') {
            input.handle_key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        }
        input.handle_key(key);
    }
}
