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
    Fecha,
    Nombre,
    Cedula,
    Empresa,
    Tipo,
    Entrada,
    Salida,
    Gafete,
    Medio,
    Ingreso,
    Egreso,
}

pub(crate) struct FormatosHistorial {
    fecha: Format,
    hora: Format,
}

impl Default for FormatosHistorial {
    fn default() -> Self {
        Self {
            fecha: Format::new().set_num_format("dd/mm/yyyy"),
            hora: Format::new().set_num_format("hh:mm"),
        }
    }
}

impl ColumnaHistorial {
    pub const ALL: [Self; 11] = [
        Self::Fecha,
        Self::Nombre,
        Self::Cedula,
        Self::Empresa,
        Self::Tipo,
        Self::Entrada,
        Self::Salida,
        Self::Gafete,
        Self::Medio,
        Self::Ingreso,
        Self::Egreso,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Fecha => "FECHA",
            Self::Cedula => "CÉDULA",
            Self::Nombre => "NOMBRE",
            Self::Empresa => "EMPRESA",
            Self::Tipo => "TIPO",
            Self::Entrada => "ENTRADA",
            Self::Salida => "SALIDA",
            Self::Gafete => "GAFETE",
            Self::Medio => "MEDIO",
            Self::Ingreso => "DA INGRESO",
            Self::Egreso => "DA SALIDA",
        }
    }

    pub const fn clave(self) -> &'static str {
        match self {
            Self::Fecha => "fecha",
            Self::Nombre => "nombre",
            Self::Cedula => "cedula",
            Self::Empresa => "empresa",
            Self::Tipo => "tipo",
            Self::Entrada => "entrada",
            Self::Salida => "salida",
            Self::Gafete => "gafete",
            Self::Medio => "medio",
            Self::Ingreso => "ingreso",
            Self::Egreso => "egreso",
        }
    }

    const fn ancho_excel(self) -> f64 {
        match self {
            Self::Fecha => 12.0,
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
            ColumnaHistorial::Fecha => {
                hoja.write_with_format(fila, indice, &ingreso_local.date_naive(), &formatos.fecha)?;
            }
            ColumnaHistorial::Nombre => {
                hoja.write_string(fila, indice, &movimiento.contratista_nombre)?;
            }
            ColumnaHistorial::Cedula => {
                // Cédula siempre es texto: Excel no debe eliminar ceros iniciales.
                hoja.write_string(fila, indice, &movimiento.cedula)?;
            }
            ColumnaHistorial::Empresa => {
                hoja.write_string(fila, indice, &movimiento.empresa_nombre)?;
            }
            ColumnaHistorial::Tipo => {
                hoja.write_string(fila, indice, tipo_texto(movimiento.tipo_ingreso))?;
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
                    hoja.write_string(fila, indice, "Activo")?;
                }
            },
            ColumnaHistorial::Gafete => {
                let valor = movimiento
                    .gafete_numero
                    .map_or_else(|| "S/G".to_owned(), |numero| numero.to_string());
                hoja.write_string(fila, indice, &valor)?;
            }
            ColumnaHistorial::Medio => {
                hoja.write_string(fila, indice, medio_texto(movimiento.medio_ingreso))?;
            }
            ColumnaHistorial::Ingreso => {
                hoja.write_string(fila, indice, &movimiento.usuario_ingreso_nombre)?;
            }
            ColumnaHistorial::Egreso => {
                hoja.write_string(
                    fila,
                    indice,
                    movimiento.usuario_salida_nombre.as_deref().unwrap_or("—"),
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
            ColumnaHistorial::Fecha,
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
