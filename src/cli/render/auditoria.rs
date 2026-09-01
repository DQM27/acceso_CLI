//! Tabla de `/auditoria` — cambios auditados de contratistas, empresas y
//! usuarios, sólo lectura. Mismo estilo visual que `/activos` (encabezado en
//! negrita, marcador `›`, pie con el total), pero con columnas fijas: a
//! diferencia de Historial no hay filtro ni exportación, así que no se
//! justifica un selector de columnas (F4) — son siempre las mismas cuatro.

use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};

use crate::cli::columnas::Columna;
use crate::database::queries::auditoria::CambioAuditado;
use crate::tiempo::a_costa_rica;

use super::estilos::{estilo_seleccion, muted};
use super::tabla::{anchos_columnas, fila_columnas};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnaAuditoria {
    Fecha,
    Entidad,
    Cambio,
    Usuario,
}

impl Columna for ColumnaAuditoria {
    const TODAS: &'static [Self] = &[Self::Fecha, Self::Entidad, Self::Cambio, Self::Usuario];

    fn etiqueta(self) -> &'static str {
        match self {
            Self::Fecha => "Fecha y hora",
            Self::Entidad => "Entidad",
            Self::Cambio => "Cambio realizado",
            Self::Usuario => "Modificado por",
        }
    }

    fn clave(self) -> &'static str {
        match self {
            Self::Fecha => "fecha",
            Self::Entidad => "entidad",
            Self::Cambio => "cambio",
            Self::Usuario => "usuario",
        }
    }
}

fn ancho_fijo(columna: ColumnaAuditoria) -> Option<usize> {
    match columna {
        ColumnaAuditoria::Fecha => Some(18),
        ColumnaAuditoria::Entidad | ColumnaAuditoria::Cambio | ColumnaAuditoria::Usuario => None,
    }
}

fn ancho_maximo(columna: ColumnaAuditoria) -> usize {
    match columna {
        ColumnaAuditoria::Entidad | ColumnaAuditoria::Usuario => 26,
        ColumnaAuditoria::Cambio => 60,
        ColumnaAuditoria::Fecha => 18,
    }
}

fn valor(item: &CambioAuditado, columna: ColumnaAuditoria) -> String {
    match columna {
        ColumnaAuditoria::Fecha => a_costa_rica(item.fecha_hora)
            .format("%d/%m/%Y %H:%M")
            .to_string(),
        ColumnaAuditoria::Entidad => format!("{} ({})", item.entidad_nombre, texto_entidad(item)),
        ColumnaAuditoria::Cambio => descripcion_cambio(item),
        ColumnaAuditoria::Usuario => item.usuario_nombre.clone(),
    }
}

fn texto_entidad(item: &CambioAuditado) -> &'static str {
    use crate::database::queries::auditoria::EntidadAuditada;
    match item.entidad {
        EntidadAuditada::Contratista => "contratista",
        EntidadAuditada::Empresa => "empresa",
        EntidadAuditada::Usuario => "usuario",
    }
}

/// Reescrito de `tui::auditoria::render::descripcion_cambio` (DEC-002/
/// DEC-014: `src/tui/` no se toca ni se comparte) — mismas etiquetas y
/// formato "Campo: antes → después", salvo `password` (marcador de evento
/// sin valores, ver `UsuarioService::cambiar_password_con_hash_auditado`):
/// ahí no hay antes/después que mostrar, sólo que ocurrió.
fn descripcion_cambio(item: &CambioAuditado) -> String {
    if item.campo == "password" {
        return "Contraseña actualizada".to_owned();
    }
    let etiqueta = match item.campo.as_str() {
        "cedula" => "Cédula",
        "nombre" => "Nombre",
        "empresa_id" => "Empresa",
        "tipo_ingreso" => "Tipo de ingreso",
        "fecha_vencimiento_praind" => "Vencimiento PRAIND",
        "es_personal_ruta" => "Personal de ruta",
        "tiene_acceso" => "Acceso",
        "rol" => "Rol",
        "activo" => "Activo",
        _ => item.campo.as_str(),
    };
    let anterior = valor_presentable(&item.campo, item.valor_anterior.as_deref());
    let nuevo = valor_presentable(&item.campo, item.valor_nuevo.as_deref());
    format!("{etiqueta}: {anterior} → {nuevo}")
}

fn valor_presentable(campo: &str, valor: Option<&str>) -> String {
    let Some(valor) = valor else {
        return if campo == "fecha_vencimiento_praind" {
            "Sin fecha".to_owned()
        } else {
            "—".to_owned()
        };
    };
    match (campo, valor) {
        ("tipo_ingreso", "IN_HOUSE") => "IN HOUSE".to_owned(),
        ("tipo_ingreso", "POR_CORREO") => "POR CORREO".to_owned(),
        ("fecha_vencimiento_praind", valor) => chrono::NaiveDate::parse_from_str(valor, "%Y-%m-%d")
            .map_or_else(
                |_| valor.to_owned(),
                |fecha| fecha.format("%d/%m/%Y").to_string(),
            ),
        ("tiene_acceso", "HABILITADO") => "Habilitado".to_owned(),
        ("tiene_acceso", "DESHABILITADO") => "Deshabilitado".to_owned(),
        ("es_personal_ruta" | "activo", "SI") => "Sí".to_owned(),
        ("es_personal_ruta" | "activo", "NO") => "No".to_owned(),
        _ => valor.to_owned(),
    }
}

pub(super) fn lineas_tabla_auditoria(
    items: &[CambioAuditado],
    total: usize,
    seleccion: usize,
    ancho: u16,
) -> Vec<Line<'static>> {
    let anchos = anchos_columnas(
        ancho,
        ColumnaAuditoria::TODAS.iter().copied(),
        ancho_fijo,
        ancho_maximo,
    );
    let mut lineas = vec![
        Line::from(Span::styled(
            format!(
                "  {}",
                fila_columnas(&anchos, |_| false, |c| c.etiqueta().to_uppercase())
            ),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("─".repeat(ancho as usize), muted())),
    ];
    for (indice, item) in items.iter().enumerate() {
        let marcador = if indice == seleccion { "› " } else { "  " };
        let texto = format!(
            "{marcador}{}",
            fila_columnas(&anchos, |_| false, |c| valor(item, c))
        );
        lineas.push(if indice == seleccion {
            Line::from(Span::styled(texto, estilo_seleccion()))
        } else {
            Line::from(texto)
        });
    }
    if items.is_empty() {
        lineas.push(Line::from(Span::styled("Sin cambios auditados", muted())));
    }
    lineas.push(Line::from(""));
    lineas.push(Line::from(Span::styled(cantidad_cambios(total), muted())));
    lineas
}

fn cantidad_cambios(total: usize) -> String {
    if total == 1 {
        "1 cambio auditado".to_string()
    } else {
        format!("{total} cambios auditados")
    }
}
