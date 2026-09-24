use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use rust_xlsxwriter::{Format, FormatAlign, FormatBorder, Worksheet, XlsxError};

use crate::{
    database::queries::ingresos::MovimientoIngresoResumen,
    models::{medio_ingreso::MedioIngreso, tipo_ingreso::TipoIngreso},
    tiempo::a_costa_rica,
};

/// Un movimiento tal como lo escribe la exportación (Excel/PDF), venga de
/// `registro_ingresos` (este dispositivo) o de `historial_sitio` (otro
/// dispositivo del sitio, ver `MIGRACION_25`). Existe porque una fila
/// remota puede traer `NULL` en campos que localmente son obligatorios
/// (filas sincronizadas antes de que la nube cargara esas columnas) --
/// antes la exportación sólo aceptaba `MovimientoIngresoResumen` y por eso
/// dejaba afuera todo lo remoto. Un campo `None` se muestra como "—", nunca
/// se inventa un valor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovimientoExportable {
    pub uuid: String,
    pub cedula: Option<String>,
    pub contratista_nombre: String,
    pub empresa_nombre: Option<String>,
    pub tipo_ingreso: Option<TipoIngreso>,
    pub medio_ingreso: Option<MedioIngreso>,
    pub fecha_hora_ingreso: DateTime<Utc>,
    pub fecha_hora_salida: Option<DateTime<Utc>>,
    pub gafete_numero: Option<i64>,
    pub placa: Option<String>,
    pub usuario_ingreso_nombre: Option<String>,
    pub usuario_salida_nombre: Option<String>,
}

impl From<MovimientoIngresoResumen> for MovimientoExportable {
    fn from(movimiento: MovimientoIngresoResumen) -> Self {
        Self {
            uuid: movimiento.uuid,
            cedula: Some(movimiento.cedula),
            contratista_nombre: movimiento.contratista_nombre,
            empresa_nombre: Some(movimiento.empresa_nombre),
            tipo_ingreso: Some(movimiento.tipo_ingreso),
            medio_ingreso: Some(movimiento.medio_ingreso),
            fecha_hora_ingreso: movimiento.fecha_hora_ingreso,
            fecha_hora_salida: movimiento.fecha_hora_salida,
            gafete_numero: movimiento.gafete_numero,
            placa: movimiento.placa,
            usuario_ingreso_nombre: Some(movimiento.usuario_ingreso_nombre),
            usuario_salida_nombre: movimiento.usuario_salida_nombre,
        }
    }
}

/// Texto de una columna opcional -- "—" cuando no vino, mismo criterio que
/// ya usaba [`ColumnaHistorial::Egreso`] sin salida. `pub` por el mismo
/// motivo que [`tipo_texto`]: el PDF de la GUI lo reusa.
pub fn o_guion(valor: Option<&str>) -> String {
    valor.unwrap_or("—").to_owned()
}

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

/// Azul claro de banda alterna ("cebra") — un tono distinto al del
/// encabezado (`D9EAF7`, ver `preparar_hoja`) para que no se confundan.
const COLOR_ZEBRA: &str = "BDD7EE";

/// Arial 10 negrita — pedido explícito del usuario para todo el archivo, no
/// sólo el encabezado (que ya era negrita por su cuenta).
fn con_fuente_base(formato: Format) -> Format {
    formato.set_font_name("Arial").set_font_size(10).set_bold()
}

/// `[fila impar, fila par]` del mismo formato — la única diferencia es el
/// fondo. `escribir_movimiento` elige el índice según `fila % 2`.
fn con_cebra(formato: Format) -> [Format; 2] {
    [formato.clone(), formato.set_background_color(COLOR_ZEBRA)]
}

pub(crate) struct FormatosHistorial {
    fecha: [Format; 2],
    hora: [Format; 2],
    /// Centrado (sin formato numérico) — columnas de texto salvo
    /// [`ColumnaHistorial::Nombre`]/[`ColumnaHistorial::Cedula`] (esas usan
    /// [`Self::texto`], alineado a la izquierda).
    centrado: [Format; 2],
    /// Mismo formato base que [`Self::centrado`] pero sin `set_align`
    /// (izquierda, el default de Excel para texto) — antes Nombre/Cédula se
    /// escribían sin ningún `Format`, lo cual ya no alcanza para que les
    /// llegue la fuente base o la cebra.
    texto: [Format; 2],
}

/// Valor ya preparado para Excel junto con la familia de formato que le
/// corresponde. Separar esta decisión del bucle de escritura evita repetir
/// la misma llamada de `Worksheet` en cada columna.
enum CeldaMovimiento {
    Fecha(NaiveDate),
    Hora(NaiveTime),
    Centrada(String),
    Texto(String),
}

fn celda_movimiento(
    movimiento: &MovimientoExportable,
    columna: ColumnaHistorial,
) -> CeldaMovimiento {
    let ingreso_local = a_costa_rica(movimiento.fecha_hora_ingreso);
    match columna {
        ColumnaHistorial::FechaIngreso => CeldaMovimiento::Fecha(ingreso_local.date_naive()),
        ColumnaHistorial::FechaSalida => movimiento.fecha_hora_salida.map_or_else(
            || CeldaMovimiento::Centrada("Activo".to_owned()),
            |salida| CeldaMovimiento::Fecha(a_costa_rica(salida).date_naive()),
        ),
        ColumnaHistorial::Nombre => CeldaMovimiento::Texto(movimiento.contratista_nombre.clone()),
        // Cédula siempre es texto: Excel no debe eliminar ceros iniciales.
        ColumnaHistorial::Cedula => CeldaMovimiento::Texto(o_guion(movimiento.cedula.as_deref())),
        ColumnaHistorial::Empresa => {
            CeldaMovimiento::Centrada(o_guion(movimiento.empresa_nombre.as_deref()))
        }
        ColumnaHistorial::Tipo => {
            CeldaMovimiento::Centrada(o_guion(movimiento.tipo_ingreso.map(tipo_texto)))
        }
        ColumnaHistorial::Entrada => CeldaMovimiento::Hora(ingreso_local.time()),
        ColumnaHistorial::Salida => movimiento.fecha_hora_salida.map_or_else(
            || CeldaMovimiento::Centrada("Activo".to_owned()),
            |salida| CeldaMovimiento::Hora(a_costa_rica(salida).time()),
        ),
        ColumnaHistorial::Gafete => CeldaMovimiento::Centrada(
            movimiento
                .gafete_numero
                .map_or_else(|| "S/G".to_owned(), |numero| numero.to_string()),
        ),
        ColumnaHistorial::Medio => CeldaMovimiento::Centrada(texto_medio(movimiento)),
        ColumnaHistorial::Ingreso => {
            CeldaMovimiento::Centrada(o_guion(movimiento.usuario_ingreso_nombre.as_deref()))
        }
        ColumnaHistorial::Egreso => {
            CeldaMovimiento::Centrada(o_guion(movimiento.usuario_salida_nombre.as_deref()))
        }
    }
}

impl Default for FormatosHistorial {
    fn default() -> Self {
        Self {
            fecha: con_cebra(con_fuente_base(
                Format::new()
                    .set_num_format("dd/mm/yyyy")
                    .set_align(FormatAlign::Center),
            )),
            hora: con_cebra(con_fuente_base(
                Format::new()
                    .set_num_format("hh:mm")
                    .set_align(FormatAlign::Center),
            )),
            centrado: con_cebra(con_fuente_base(
                Format::new().set_align(FormatAlign::Center),
            )),
            texto: con_cebra(con_fuente_base(Format::new())),
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
        Self::ALL
            .into_iter()
            .find(|columna| columna.clave() == clave)
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

/// Encabezado de columna de todas las exportaciones a Excel: Arial 10
/// negrita, centrado, con borde y fondo celeste.
fn formato_encabezado() -> Format {
    con_fuente_base(
        Format::new()
            .set_align(FormatAlign::Center)
            .set_border(FormatBorder::Thin)
            .set_background_color("D9EAF7"),
    )
}

pub(crate) fn preparar_hoja(
    hoja: &mut Worksheet,
    columnas: &[ColumnaHistorial],
) -> Result<(), XlsxError> {
    let encabezado = formato_encabezado();

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
    movimiento: &MovimientoExportable,
    formatos: &FormatosHistorial,
) -> Result<(), XlsxError> {
    // Alterna cebra según la fila real de Excel (1 = primera fila de datos,
    // ver `application/historial.rs`) — no un contador propio, para que dos
    // llamadas consecutivas con la misma `fila` (no debería pasar, pero si
    // pasara) no desincronicen el patrón del resto de la hoja.
    let variante = (fila % 2) as usize;

    for (indice, columna) in columnas.iter().copied().enumerate() {
        let indice = u16::try_from(indice).unwrap_or(u16::MAX);
        match celda_movimiento(movimiento, columna) {
            CeldaMovimiento::Fecha(valor) => {
                hoja.write_with_format(fila, indice, &valor, &formatos.fecha[variante])?;
            }
            CeldaMovimiento::Hora(valor) => {
                hoja.write_with_format(fila, indice, &valor, &formatos.hora[variante])?;
            }
            CeldaMovimiento::Centrada(valor) => {
                hoja.write_string_with_format(fila, indice, &valor, &formatos.centrado[variante])?;
            }
            CeldaMovimiento::Texto(valor) => {
                hoja.write_string_with_format(fila, indice, &valor, &formatos.texto[variante])?;
            }
        }
    }
    Ok(())
}

/// Columna de una tabla genérica a exportar (ver [`escribir_tabla_generica`]):
/// el título tal cual se ve en la grilla y si el dato va alineado a la
/// izquierda (texto libre, ej. nombres) o centrado (el resto).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct ColumnaTabla {
    pub titulo: String,
    pub izquierda: bool,
}

/// Hoja de Excel con cualquier tabla ya formateada como texto -- la usan
/// las grillas de escritorio que no tienen un exportador propio (historial
/// de proveedores y de KOF): la grilla manda exactamente lo que muestra
/// (columnas visibles, filas filtradas y ordenadas, valores ya formateados)
/// y acá sólo se le da el mismo estilo que al Historial de contratistas
/// (encabezado, cebra, Arial 10 negrita, autofiltro). El ancho de cada
/// columna sale del texto más largo, entre 8 y 50 caracteres.
pub(crate) fn escribir_tabla_generica(
    hoja: &mut Worksheet,
    columnas: &[ColumnaTabla],
    filas: &[Vec<String>],
) -> Result<(), XlsxError> {
    let encabezado = formato_encabezado();
    let centrado = con_cebra(con_fuente_base(Format::new().set_align(FormatAlign::Center)));
    let izquierda = con_cebra(con_fuente_base(Format::new()));

    hoja.set_name("Movimientos")?;
    hoja.set_freeze_panes(1, 0)?;
    for (indice, columna) in columnas.iter().enumerate() {
        let largo = filas
            .iter()
            .filter_map(|fila| fila.get(indice))
            .map(|valor| valor.chars().count())
            .chain(std::iter::once(columna.titulo.chars().count()))
            .max()
            .unwrap_or(0);
        let ancho = u32::try_from(largo.clamp(8, 50) + 3).unwrap_or(53);
        let indice = u16::try_from(indice).unwrap_or(u16::MAX);
        hoja.set_column_width(indice, f64::from(ancho))?;
        hoja.write_string_with_format(0, indice, &columna.titulo, &encabezado)?;
    }
    for (numero, fila) in filas.iter().enumerate() {
        let fila_excel = u32::try_from(numero + 1).unwrap_or(u32::MAX);
        let variante = (fila_excel % 2) as usize;
        for (indice, columna) in columnas.iter().enumerate() {
            let valor = fila.get(indice).map_or("", String::as_str);
            let formato = if columna.izquierda {
                &izquierda[variante]
            } else {
                &centrado[variante]
            };
            let indice = u16::try_from(indice).unwrap_or(u16::MAX);
            hoja.write_string_with_format(fila_excel, indice, valor, formato)?;
        }
    }
    if !columnas.is_empty() {
        let ultima = u16::try_from(columnas.len() - 1).unwrap_or(u16::MAX);
        hoja.autofilter(0, 0, u32::try_from(filas.len()).unwrap_or(u32::MAX), ultima)?;
    }
    Ok(())
}

/// `pub` (no `pub(crate)`) a propósito — la exportación a PDF de la GUI
/// (`desktop/src-tauri`, un crate distinto) reusa el mismo texto que ya
/// usa la exportación a Excel en vez de duplicar el `match`.
pub const fn tipo_texto(tipo: TipoIngreso) -> &'static str {
    match tipo {
        TipoIngreso::Praind => "PRAIND",
        TipoIngreso::InHouse => "IN-HOUSE",
        TipoIngreso::PorCorreo => "POR CORREO",
        TipoIngreso::Swat => "SWAT",
    }
}

/// `pub` por el mismo motivo que [`tipo_texto`].
pub const fn medio_texto(medio: MedioIngreso) -> &'static str {
    match medio {
        MedioIngreso::Caminando => "Caminando",
        MedioIngreso::Vehiculo => "Vehículo",
    }
}

/// Igual que [`medio_texto`], pero muestra la placa en vez del texto
/// genérico "Vehículo" cuando hay una guardada (`MIGRACION_49`) -- pedido
/// explícito del usuario, 2026-09-21. Cae a "Vehículo" si no hay placa
/// (dato viejo pre-migración): nunca queda una celda vacía. `pub` por el
/// mismo motivo que [`tipo_texto`]/[`medio_texto`] -- la exportación a PDF
/// de la GUI reusa esta función en vez de duplicar el criterio.
pub fn medio_texto_con_placa(medio: MedioIngreso, placa: Option<&str>) -> String {
    match (medio, placa) {
        (MedioIngreso::Vehiculo, Some(placa)) if !placa.trim().is_empty() => placa.to_owned(),
        _ => medio_texto(medio).to_owned(),
    }
}

/// [`medio_texto_con_placa`] sobre un [`MovimientoExportable`] -- "—" si
/// el medio no vino (fila remota vieja). `pub` por el mismo motivo que
/// [`tipo_texto`]: el PDF de la GUI la reusa.
pub fn texto_medio(movimiento: &MovimientoExportable) -> String {
    movimiento.medio_ingreso.map_or_else(
        || "—".to_owned(),
        |medio| medio_texto_con_placa(medio, movimiento.placa.as_deref()),
    )
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, Utc};

    use super::*;
    use crate::models::registro_ingreso::{ResultadoIngresoRegistrado, VERSION_REGLAS_ACCESO};

    fn movimiento() -> MovimientoIngresoResumen {
        MovimientoIngresoResumen {
            registro_id: 1,
            uuid: "uuid-movimiento".into(),
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
            placa: None,
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
                &movimiento().into(),
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

    #[test]
    fn escribe_una_tabla_generica_con_filas_mas_cortas_que_las_columnas() {
        let directorio = tempfile::tempdir().unwrap();
        let destino = directorio.path().join("tabla.xlsx");
        let columnas = vec![
            ColumnaTabla { titulo: "NOMBRE".into(), izquierda: true },
            ColumnaTabla { titulo: "GAFETE".into(), izquierda: false },
        ];
        // La segunda fila trae una celda menos: se escribe vacía, no falla.
        let filas = vec![
            vec!["Ana Solano".to_owned(), "S/G".to_owned()],
            vec!["Beto Rojas".to_owned()],
        ];
        let mut libro = rust_xlsxwriter::Workbook::new();
        escribir_tabla_generica(libro.add_worksheet(), &columnas, &filas).unwrap();
        libro.save(&destino).unwrap();

        let bytes = std::fs::read(destino).unwrap();
        assert!(bytes.starts_with(b"PK"), "XLSX debe ser un contenedor ZIP");
    }

    #[test]
    fn medio_texto_con_placa_muestra_la_placa_en_vehiculo() {
        assert_eq!(
            medio_texto_con_placa(MedioIngreso::Vehiculo, Some("ABC123")),
            "ABC123"
        );
    }

    #[test]
    fn medio_texto_con_placa_cae_a_vehiculo_sin_placa() {
        assert_eq!(
            medio_texto_con_placa(MedioIngreso::Vehiculo, None),
            "Vehículo"
        );
        assert_eq!(
            medio_texto_con_placa(MedioIngreso::Vehiculo, Some("   ")),
            "Vehículo"
        );
    }

    #[test]
    fn medio_texto_con_placa_ignora_placa_en_caminando() {
        // No debería pasar (el CHECK de MIGRACION_49 lo impide), pero si
        // llegara un dato así de todos modos no debe mostrarse la placa.
        assert_eq!(
            medio_texto_con_placa(MedioIngreso::Caminando, Some("ABC123")),
            "Caminando"
        );
    }
}
