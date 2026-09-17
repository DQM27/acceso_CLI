use chrono::NaiveDate;

use control_acceso::models::encargado_ruta::EncargadoRuta;
use control_acceso::models::vehiculo_ruta::VehiculoRuta;
use control_acceso::services::ruta_service::SolicitudSalidaRuta;

/// Espejo de `VehiculoRuta` sin `id` -- el comando lo recibe aparte (crear
/// pide `0`, actualizar pide el real), mismo criterio que
/// `DatosContratistaEntrada`/`actualizar_contratista`.
#[derive(serde::Deserialize)]
pub struct DatosVehiculoRutaEntrada {
    pub numero_unidad: Option<String>,
    pub placa: String,
    pub activo: bool,
}

impl DatosVehiculoRutaEntrada {
    pub fn construir(self, id: i64) -> VehiculoRuta {
        VehiculoRuta {
            id,
            numero_unidad: self
                .numero_unidad
                .map(|texto| texto.trim().to_owned())
                .filter(|texto| !texto.is_empty()),
            placa: self.placa.trim().to_owned(),
            activo: self.activo,
        }
    }
}

/// Espejo de `EncargadoRuta` sin `id`, mismo criterio que
/// `DatosVehiculoRutaEntrada`. Sin campo `cedula` -- el catálogo KOF se
/// precarga sólo con nombre y código de empleado (pedido explícito del
/// usuario, 2026-09-15), el formulario de escritorio no ofrece capturarla.
#[derive(serde::Deserialize)]
pub struct DatosEncargadoRutaEntrada {
    pub codigo_empleado: String,
    pub nombre: String,
    pub activo: bool,
}

impl DatosEncargadoRutaEntrada {
    pub fn construir(self, id: i64) -> EncargadoRuta {
        EncargadoRuta {
            id,
            codigo_empleado: self.codigo_empleado.trim().to_owned(),
            nombre: self.nombre.trim().to_owned(),
            cedula: None,
            activo: self.activo,
        }
    }
}

/// Espejo de `SolicitudSalidaRuta` sin `usuario_salida_id`/`fecha_hora_salida`
/// -- ver el doc-comment de `AppCore::registrar_salida_ruta`
/// (`src/application/rutas.rs`): esos dos campos siempre se pisan del lado
/// del núcleo, nunca llegan del webview.
#[derive(serde::Deserialize)]
pub struct SolicitudSalidaRutaEntrada {
    pub vehiculo_placa: String,
    pub vehiculo_numero_unidad: Option<String>,
    pub encargado_nombre: String,
    pub encargado_codigo_empleado: Option<String>,
    pub numero_ruta: i64,
    pub sub_numero: i64,
    pub numero_documento: String,
    pub fecha_documento: NaiveDate,
    pub tiene_correo_autorizacion: bool,
}

impl SolicitudSalidaRutaEntrada {
    pub fn construir(self) -> SolicitudSalidaRuta {
        SolicitudSalidaRuta {
            vehiculo_placa: self.vehiculo_placa,
            vehiculo_numero_unidad: self.vehiculo_numero_unidad,
            encargado_nombre: self.encargado_nombre,
            encargado_codigo_empleado: self.encargado_codigo_empleado,
            numero_ruta: self.numero_ruta,
            sub_numero: self.sub_numero,
            numero_documento: self.numero_documento,
            fecha_documento: self.fecha_documento,
            tiene_correo_autorizacion: self.tiene_correo_autorizacion,
            // Placeholders -- `AppCore::registrar_salida_ruta` los pisa con
            // el actor/reloj reales de la transacción, nunca se leen.
            usuario_salida_id: 0,
            fecha_hora_salida: chrono::Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_numero_de_unidad_en_blanco_se_convierte_en_ninguno() {
        let vehiculo = DatosVehiculoRutaEntrada {
            numero_unidad: Some("   ".to_owned()),
            placa: "C12345".to_owned(),
            activo: true,
        }
        .construir(0);

        assert_eq!(vehiculo.numero_unidad, None);
    }

    #[test]
    fn la_placa_se_recorta_de_espacios() {
        let vehiculo = DatosVehiculoRutaEntrada {
            numero_unidad: None,
            placa: "  C12345  ".to_owned(),
            activo: true,
        }
        .construir(7);

        assert_eq!(vehiculo.id, 7);
        assert_eq!(vehiculo.placa, "C12345");
    }

    #[test]
    fn el_encargado_nunca_lleva_cedula() {
        let encargado = DatosEncargadoRutaEntrada {
            codigo_empleado: "5040017".to_owned(),
            nombre: "Michael Araya Retana".to_owned(),
            activo: true,
        }
        .construir(0);

        assert_eq!(encargado.cedula, None);
    }
}
