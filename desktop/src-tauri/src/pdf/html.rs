//! Arma el documento HTML que después se convierte a PDF (`generador.rs`).
//! Módulo puro — sin Tauri ni COM, testeable igual que
//! `historial/exportacion.rs` en el núcleo, que es el mismo tipo de dato
//! con el mismo criterio de columnas/orden.

use std::fmt::Write as _;

use chrono::Utc;
use control_acceso::historial::exportacion::{
    ColumnaHistorial, ColumnaTabla, MovimientoExportable, o_guion, texto_medio, tipo_texto,
};
use control_acceso::tiempo::a_costa_rica;

/// Paleta CLARA de `desktop/src/index.css` (light) a propósito, aunque la
/// app esté en modo oscuro — un PDF siempre se lee/imprime en claro. Mismos
/// valores que el prototipo ya aprobado por el usuario.
const ESTILO: &str = r"
  :root {
    --acento: #087f91;
    --texto: #172026;
    --muted: #63717c;
    --borde: #d8e0e5;
    --panel-suave: #eef3f5;
    --zebra: #d9eaf7;
  }
  /* Horizontal a propósito — con 12 columnas posibles, vertical queda
     apretado (confirmado contra una exportación real: los nombres largos
     se partían en 4-5 líneas). */
  @page { size: letter landscape; margin: 1.1cm 0.9cm; }
  * { box-sizing: border-box; }
  body {
    margin: 0;
    font-family: Arial, Helvetica, sans-serif;
    font-size: 8.5pt;
    color: var(--texto);
    -webkit-print-color-adjust: exact;
    print-color-adjust: exact;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    border-bottom: 2px solid var(--acento);
    padding: 0 0.3cm 0.5em;
    margin-bottom: 0.7em;
  }
  header h1 { margin: 0; font-size: 14pt; color: var(--acento); }
  header .subtitulo { margin: 0.15em 0 0; font-size: 8.5pt; color: var(--muted); }
  .meta { text-align: right; font-size: 8pt; color: var(--muted); line-height: 1.5; }
  .meta strong { color: var(--texto); }
  table { width: 100%; border-collapse: collapse; }
  thead { display: table-header-group; }
  tr { page-break-inside: avoid; }
  th, td { border: 1px solid var(--borde); padding: 0.28em 0.4em; text-align: center; }
  th { background: var(--panel-suave); font-weight: bold; font-size: 7.5pt; }
  td.izquierda, th.izquierda { text-align: left; }
  tbody tr:nth-child(even) { background: var(--zebra); }
";

/// Columnas que van alineadas a la izquierda (texto libre) — el resto
/// (fechas/horas/números/enums cortos) centrado, mismo criterio visual que
/// el prototipo aprobado.
fn es_columna_izquierda(columna: ColumnaHistorial) -> bool {
    matches!(
        columna,
        ColumnaHistorial::Nombre
            | ColumnaHistorial::Cedula
            | ColumnaHistorial::Empresa
            | ColumnaHistorial::Ingreso
            | ColumnaHistorial::Egreso
    )
}

/// Mismo texto por columna que ya escribe `historial/exportacion.rs` a
/// Excel (`escribir_movimiento`) — no una segunda fuente de verdad de cómo
/// se ve cada dato.
fn valor_columna(columna: ColumnaHistorial, movimiento: &MovimientoExportable) -> String {
    let ingreso_local = a_costa_rica(movimiento.fecha_hora_ingreso);
    match columna {
        ColumnaHistorial::FechaIngreso => ingreso_local.format("%d/%m/%Y").to_string(),
        ColumnaHistorial::FechaSalida => movimiento.fecha_hora_salida.map_or_else(
            || "Activo".to_string(),
            |s| a_costa_rica(s).format("%d/%m/%Y").to_string(),
        ),
        ColumnaHistorial::Nombre => movimiento.contratista_nombre.clone(),
        ColumnaHistorial::Cedula => o_guion(movimiento.cedula.as_deref()),
        ColumnaHistorial::Empresa => o_guion(movimiento.empresa_nombre.as_deref()),
        ColumnaHistorial::Tipo => o_guion(movimiento.tipo_ingreso.map(tipo_texto)),
        ColumnaHistorial::Entrada => ingreso_local.format("%H:%M").to_string(),
        ColumnaHistorial::Salida => movimiento.fecha_hora_salida.map_or_else(
            || "Activo".to_string(),
            |s| a_costa_rica(s).format("%H:%M").to_string(),
        ),
        ColumnaHistorial::Gafete => movimiento
            .gafete_numero
            .map_or_else(|| "S/G".to_string(), |numero| numero.to_string()),
        ColumnaHistorial::Medio => texto_medio(movimiento),
        ColumnaHistorial::Ingreso => o_guion(movimiento.usuario_ingreso_nombre.as_deref()),
        ColumnaHistorial::Egreso => o_guion(movimiento.usuario_salida_nombre.as_deref()),
    }
}

/// Escapa lo mínimo indispensable para HTML — todo lo que entra acá viene
/// de datos reales (nombres, empresas, nombre de sesión, descripción del
/// filtro) y nunca se validó como "sin `<`/`&`", a diferencia del XLSX
/// (que no tiene este riesgo porque no interpreta marcado).
fn escapar(texto: &str) -> String {
    texto
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn generar_html(
    movimientos: &[MovimientoExportable],
    columnas: &[ColumnaHistorial],
    generado_por: &str,
    filtro_descripcion: &str,
) -> String {
    let clase = |columna: ColumnaHistorial| {
        if es_columna_izquierda(columna) {
            " class=\"izquierda\""
        } else {
            ""
        }
    };

    let mut encabezados = String::new();
    for &columna in columnas {
        write!(
            encabezados,
            "<th{}>{}</th>",
            clase(columna),
            escapar(columna.label())
        )
        .expect("escribir en String no falla");
    }

    let mut filas = String::new();
    for movimiento in movimientos {
        filas.push_str("<tr>");
        for &columna in columnas {
            write!(
                filas,
                "<td{}>{}</td>",
                clase(columna),
                escapar(&valor_columna(columna, movimiento))
            )
            .expect("escribir en String no falla");
        }
        filas.push_str("</tr>");
    }

    documento(
        "Historial de Movimientos",
        &encabezados,
        &filas,
        generado_por,
        filtro_descripcion,
    )
}

/// Mismo documento que [`generar_html`] pero para cualquier tabla ya
/// formateada del lado de la GUI (títulos y valores como texto, tal cual se
/// ven en la grilla) -- historial de proveedores y de KOF, que no tienen un
/// exportador propio. `titulo` va en el encabezado del PDF.
pub fn generar_html_tabla(
    titulo: &str,
    columnas: &[ColumnaTabla],
    filas: &[Vec<String>],
    generado_por: &str,
    filtro_descripcion: &str,
) -> String {
    let clase = |columna: &ColumnaTabla| {
        if columna.izquierda {
            " class=\"izquierda\""
        } else {
            ""
        }
    };

    let mut encabezados = String::new();
    for columna in columnas {
        write!(encabezados, "<th{}>{}</th>", clase(columna), escapar(&columna.titulo))
            .expect("escribir en String no falla");
    }

    let mut cuerpo = String::new();
    for fila in filas {
        cuerpo.push_str("<tr>");
        for (indice, columna) in columnas.iter().enumerate() {
            let valor = fila.get(indice).map_or("", String::as_str);
            write!(cuerpo, "<td{}>{}</td>", clase(columna), escapar(valor))
                .expect("escribir en String no falla");
        }
        cuerpo.push_str("</tr>");
    }

    documento(titulo, &encabezados, &cuerpo, generado_por, filtro_descripcion)
}

/// Esqueleto común de los PDF (encabezado con título, filtro, quién lo
/// generó y cuándo, más la tabla) -- `encabezados`/`filas` ya vienen
/// escapados; el resto se escapa acá.
fn documento(
    titulo: &str,
    encabezados: &str,
    filas: &str,
    generado_por: &str,
    filtro_descripcion: &str,
) -> String {
    let generado_en = a_costa_rica(Utc::now())
        .format("%d/%m/%Y %H:%M")
        .to_string();

    format!(
        r#"<!doctype html>
<html lang="es">
<head>
<meta charset="utf-8" />
<title>{titulo}</title>
<style>{ESTILO}</style>
</head>
<body>
  <header>
    <div>
      <h1>{titulo}</h1>
      <p class="subtitulo">{filtro}</p>
    </div>
    <div class="meta">
      Generado por: <strong>{generado_por}</strong><br />
      {generado_en}
    </div>
  </header>
  <table>
    <thead><tr>{encabezados}</tr></thead>
    <tbody>{filas}</tbody>
  </table>
</body>
</html>"#,
        titulo = escapar(titulo),
        filtro = escapar(filtro_descripcion),
        generado_por = escapar(generado_por),
    )
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use control_acceso::models::registro_ingreso::{
        ResultadoIngresoRegistrado, VERSION_REGLAS_ACCESO,
    };
    use control_acceso::models::{medio_ingreso::MedioIngreso, tipo_ingreso::TipoIngreso};

    use control_acceso::database::queries::ingresos::MovimientoIngresoResumen;

    use super::*;

    fn movimiento() -> MovimientoExportable {
        MovimientoIngresoResumen {
            registro_id: 1,
            uuid: "00000000-0000-0000-0000-000000000001".into(),
            contratista_id: 2,
            cedula: "001010101".into(),
            contratista_nombre: "María <Pérez> & \"Ruiz\"".into(),
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
        .into()
    }

    #[test]
    fn incluye_encabezado_columnas_y_datos() {
        let columnas = [ColumnaHistorial::Nombre, ColumnaHistorial::Gafete];
        let html = generar_html(
            &[movimiento()],
            &columnas,
            "Daniel Quintana",
            "Todo el historial",
        );

        assert!(html.contains("<title>Historial de Movimientos</title>"));
        assert!(html.contains("Daniel Quintana"));
        assert!(html.contains("Todo el historial"));
        assert!(html.contains("NOMBRE"));
        assert!(html.contains("GAFETE"));
        assert!(html.contains('7'));
    }

    #[test]
    fn escapa_html_en_datos_de_usuario_para_no_inyectar_marcado() {
        let columnas = [ColumnaHistorial::Nombre];
        let html = generar_html(&[movimiento()], &columnas, "Daniel", "Todo el historial");

        assert!(!html.contains("<Pérez>"));
        assert!(html.contains("&lt;Pérez&gt;"));
        assert!(html.contains("&quot;Ruiz&quot;"));
    }

    #[test]
    fn columna_activa_sin_salida_muestra_activo_no_vacio() {
        let columnas = [ColumnaHistorial::Salida, ColumnaHistorial::FechaSalida];
        let html = generar_html(&[movimiento()], &columnas, "Daniel", "Todo el historial");

        assert_eq!(html.matches("Activo").count(), 2);
    }

    /// Una fila remota vieja (`historial_sitio` sin tipo/medio/cédula)
    /// igual sale en el PDF, con "—" en lo que no vino -- nunca se inventa.
    #[test]
    fn fila_remota_sin_datos_opcionales_muestra_guion() {
        let mut remoto = movimiento();
        remoto.cedula = None;
        remoto.tipo_ingreso = None;
        remoto.medio_ingreso = None;
        let columnas = [
            ColumnaHistorial::Cedula,
            ColumnaHistorial::Tipo,
            ColumnaHistorial::Medio,
        ];
        let html = generar_html(&[remoto], &columnas, "Daniel", "Todo el historial");

        assert_eq!(html.matches("<td>—</td>").count(), 2);
        assert!(html.contains("<td class=\"izquierda\">—</td>"));
    }

    /// Tabla genérica (historial de proveedores/KOF): título propio,
    /// alineación por columna y todo escapado.
    #[test]
    fn tabla_generica_usa_su_titulo_y_escapa_los_datos() {
        let columnas = vec![
            ColumnaTabla { titulo: "NOMBRE".into(), izquierda: true },
            ColumnaTabla { titulo: "GAFETE".into(), izquierda: false },
        ];
        let filas = vec![vec!["Ana <Solano>".to_owned(), "S/G".to_owned()]];
        let html = generar_html_tabla(
            "Historial de Proveedores",
            &columnas,
            &filas,
            "Daniel",
            "Filtro: Hoy",
        );

        assert!(html.contains("<title>Historial de Proveedores</title>"));
        assert!(html.contains("<th class=\"izquierda\">NOMBRE</th>"));
        assert!(html.contains("<td class=\"izquierda\">Ana &lt;Solano&gt;</td>"));
        assert!(html.contains("<td>S/G</td>"));
    }
}
