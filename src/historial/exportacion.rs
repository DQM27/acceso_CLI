use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Worksheet, XlsxError};

use crate::{
    database::queries::ingresos::MovimientoIngresoResumen,
    models::{medio_ingreso::MedioIngreso, tipo_ingreso::TipoIngreso},
    tiempo::a_costa_rica,
};

/// Columnas que el operador puede mostrar tanto en la tabla clásica como en
/// una exportación de Historial. Mantener un único enum evita que F4 y el
/// archivo XLSX diverjan con el tiempo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnaHistorial {
    FechaIngreso,
    Nombre,
    Cedula,
    Empresa,
    Tipo,
    Entrada,
    /// Fecha del `fecha_hora_salida` — columna separada de [`Self::FechaIngreso`]
    /// porque un movimiento puede entrar un día y salir otro (turno nocturno);
    /// antes no existía y la exportación no tenía forma de reflejarlo, sólo
    /// la hora (ver [`Self::Salida`]).
    FechaSalida,
    Salida,
    Gafete,
    Medio,
    Ingreso,
    Egreso,
}

pub(crate) struct FormatosHistorial {
    fecha: Format,
    hora: Format,
    /// Centrado plano (sin formato numérico) — todas las columnas de texto
    /// salvo [`ColumnaHistorial::Nombre`]/[`ColumnaHistorial::Cedula`]
    /// (esas quedan alineadas a la izquierda, que es lo que Excel ya hace
    /// por defecto con texto, así que no necesitan `Format` propio).
    centrado: Format,
}

impl Default for FormatosHistorial {
    fn default() -> Self {
        Self {
            fecha: Format::new()
                .set_num_format("dd/mm/yyyy")
                .set_align(FormatAlign::Center),
            hora: Format::new()
                .set_num_format("hh:mm")
                .set_align(FormatAlign::Center),
            centrado: Format::new().set_align(FormatAlign::Center),
        }
    }
}

impl ColumnaHistorial {
    pub const ALL: [Self; 12] = [
        Self::FechaIngreso,
        Self::Nombre,
        Self::Cedula,
        Self::Empresa,
        Self::Tipo,
        Self::Entrada,
        Self::FechaSalida,
        Self::Salida,
        Self::Gafete,
        Self::Medio,
        Self::Ingreso,
        Self::Egreso,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::FechaIngreso => "FECHA INGRESO",
            Self::Cedula => "CÉDULA",
            Self::Nombre => "NOMBRE",
            Self::Empresa => "EMPRESA",
            Self::Tipo => "TIPO",
            Self::Entrada => "ENTRADA",
            Self::FechaSalida => "FECHA SALIDA",
            Self::Salida => "SALIDA",
            Self::Gafete => "GAFETE",
            Self::Medio => "MEDIO",
            Self::Ingreso => "DA INGRESO",
            Self::Egreso => "DA SALIDA",
        }
    }

    pub const fn clave(self) -> &'static str {
        match self {
            Self::FechaIngreso => "fecha",
            Self::Nombre => "nombre",
            Self::Cedula => "cedula",
            Self::Empresa => "empresa",
            Self::Tipo => "tipo",
            Self::Entrada => "entrada",
            Self::FechaSalida => "fecha_salida",
            Self::Salida => "salida",
            Self::Gafete => "gafete",
            Self::Medio => "medio",
            Self::Ingreso => "ingreso",
            Self::Egreso => "egreso",
        }
    }

    /// Inverso de [`Self::clave`] — la GUI manda qué columnas tiene visibles
    /// como claves de texto (mismo identificador que usan sus `colId`/
    /// `field` de AG Grid) en vez de un enum que no puede cruzar el borde de
    /// Tauri sin duplicar este tipo en TypeScript. `None` si no matchea
    /// ninguna — quien llama decide si eso es un error o simplemente se
    /// ignora esa clave.
    pub fn from_clave(clave: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|columna| columna.clave() == clave)
    }

    const fn ancho_excel(self) -> f64 {
        match self {
            Self::FechaIngreso | Self::FechaSalida => 12.0,
            Self::Nombre => 30.0,
            Self::Cedula => 18.0,
            Self::Empresa => 26.0,
            Self::Tipo => 15.0,
            Self::Entrada | Self::Salida => 11.0,
            Self::Gafete => 10.0,
            Self::Medio => 14.0,
            Self::Ingreso | Self::Egreso => 24.0,
        }
    }
}

pub(crate) fn preparar_hoja(
    hoja: &mut Worksheet,
    columnas: &[ColumnaHistorial],
) -> Result<(), XlsxError> {
    let encabezado = Format::new()
        .set_bold()
        .set_align(FormatAlign::Center)
        .set_border(FormatBorder::Thin)
        .set_background_color("D9EAF7");

    hoja.set_name("Movimientos")?;
    hoja.set_freeze_panes(1, 0)?;
    for (indice, columna) in columnas.iter().copied().enumerate() {
        let indice = u16::try_from(indice).unwrap_or(u16::MAX);
        hoja.set_column_width(indice, columna.ancho_excel())?;
        hoja.write_string_with_format(0, indice, columna.label(), &encabezado)?;
    }
    Ok(())
}

pub(crate) fn escribir_movimiento(
    hoja: &mut Worksheet,
    fila: u32,
    columnas: &[ColumnaHistorial],
    movimiento: &MovimientoIngresoResumen,
    formatos: &FormatosHistorial,
) -> Result<(), XlsxError> {
    let ingreso_local = a_costa_rica(movimiento.fecha_hora_ingreso);

    for (indice, columna) in columnas.iter().copied().enumerate() {
        let indice = u16::try_from(indice).unwrap_or(u16::MAX);
        match columna {
            ColumnaHistorial::FechaIngreso => {
                hoja.write_with_format(fila, indice, &ingreso_local.date_naive(), &formatos.fecha)?;
            }
            ColumnaHistorial::FechaSalida => match movimiento.fecha_hora_salida {
                Some(salida) => {
                    hoja.write_with_format(
                        fila,
                        indice,
                        &a_costa_rica(salida).date_naive(),
                        &formatos.fecha,
                    )?;
                }
                None => {
                    hoja.write_string_with_format(fila, indice, "Activo", &formatos.centrado)?;
                }
            },
            ColumnaHistorial::Nombre => {
                hoja.write_string(fila, indice, &movimiento.contratista_nombre)?;
            }
            ColumnaHistorial::Cedula => {
                // Cédula siempre es texto: Excel no debe eliminar ceros iniciales.
                hoja.write_string(fila, indice, &movimiento.cedula)?;
            }
            ColumnaHistorial::Empresa => {
                hoja.write_string_with_format(
                    fila,
                    indice,
                    &movimiento.empresa_nombre,
                    &formatos.centrado,
                )?;
            }
            ColumnaHistorial::Tipo => {
                hoja.write_string_with_format(
                    fila,
                    indice,
                    tipo_texto(movimiento.tipo_ingreso),
                    &formatos.centrado,
                )?;
            }
            ColumnaHistorial::Entrada => {
                hoja.write_with_format(fila, indice, &ingreso_local.time(), &formatos.hora)?;
            }
            ColumnaHistorial::Salida => match movimiento.fecha_hora_salida {
                Some(salida) => {
                    hoja.write_with_format(
                        fila,
                        indice,
                        &a_costa_rica(salida).time(),
                        &formatos.hora,
                    )?;
                }
                None => {
                    hoja.write_string_with_format(fila, indice, "Activo", &formatos.centrado)?;
                }
            },
            ColumnaHistorial::Gafete => {
                let valor = movimiento
                    .gafete_numero
                    .map_or_else(|| "S/G".to_owned(), |numero| numero.to_string());
                hoja.write_string_with_format(fila, indice, &valor, &formatos.centrado)?;
            }
            ColumnaHistorial::Medio => {
                hoja.write_string_with_format(
                    fila,
                    indice,
                    medio_texto(movimiento.medio_ingreso),
                    &formatos.centrado,
                )?;
            }
            ColumnaHistorial::Ingreso => {
                hoja.write_string_with_format(
                    fila,
                    indice,
                    &movimiento.usuario_ingreso_nombre,
                    &formatos.centrado,
                )?;
            }
            ColumnaHistorial::Egreso => {
                hoja.write_string_with_format(
                    fila,
                    indice,
                    movimiento.usuario_salida_nombre.as_deref().unwrap_or("—"),
                    &formatos.centrado,
                )?;
            }
        }
    }
    Ok(())
}

pub(crate) const fn tipo_texto(tipo: TipoIngreso) -> &'static str {
    match tipo {
        TipoIngreso::Praind => "PRAIND",
        TipoIngreso::InHouse => "IN-HOUSE",
        TipoIngreso::PorCorreo => "POR CORREO",
        TipoIngreso::Swat => "SWAT",
    }
}

pub(crate) const fn medio_texto(medio: MedioIngreso) -> &'static str {
    match medio {
        MedioIngreso::Caminando => "Caminando",
        MedioIngreso::Vehiculo => "Vehículo",
    }
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, Utc};

    use super::*;
    use crate::models::registro_ingreso::{ResultadoIngresoRegistrado, VERSION_REGLAS_ACCESO};

    fn movimiento() -> MovimientoIngresoResumen {
        MovimientoIngresoResumen {
            registro_id: 1,
            contratista_id: 2,
            cedula: "001010101".into(),
            contratista_nombre: "=1+1".into(),
            empresa_nombre: "Brisas".into(),
            tipo_ingreso: TipoIngreso::Praind,
            medio_ingreso: MedioIngreso::Caminando,
            fecha_hora_ingreso: chrono::DateTime::from_naive_utc_and_offset(
                NaiveDate::from_ymd_opt(2026, 8, 20)
                    .unwrap()
                    .and_hms_opt(14, 30, 0)
                    .unwrap(),
                Utc,
            ),
            fecha_hora_salida: None,
            gafete_numero: Some(7),
            usuario_ingreso_nombre: "Quintana".into(),
            usuario_salida_nombre: None,
            resultado_acceso: ResultadoIngresoRegistrado::Permitido,
            motivo_resultado: None,
            reglas_version: VERSION_REGLAS_ACCESO,
            empresa_activa_snapshot: true,
        }
    }

    #[test]
    fn genera_un_xlsx_real_con_las_columnas_solicitadas() {
        let directorio = tempfile::tempdir().unwrap();
        let destino = directorio.path().join("historial.xlsx");
        let columnas = [
            ColumnaHistorial::FechaIngreso,
            ColumnaHistorial::Cedula,
            ColumnaHistorial::Nombre,
            ColumnaHistorial::Salida,
        ];
        let mut libro = rust_xlsxwriter::Workbook::new();
        {
            let hoja = libro.add_worksheet_with_constant_memory();
            preparar_hoja(hoja, &columnas).unwrap();
            escribir_movimiento(
                hoja,
                1,
                &columnas,
                &movimiento(),
                &FormatosHistorial::default(),
            )
            .unwrap();
            hoja.autofilter(0, 0, 1, 3).unwrap();
        }
        libro.save(&destino).unwrap();

        let bytes = std::fs::read(destino).unwrap();
        assert!(bytes.starts_with(b"PK"), "XLSX debe ser un contenedor ZIP");
        assert!(bytes.len() > 1_000, "el libro no debe quedar vacío");
    }
}
