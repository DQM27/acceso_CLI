use control_acceso::application::NuevoIngresoCorreo;

/// Espejo de los datos de `application::registrar_ingreso_correo_verificado`
/// -- mismo criterio que `SolicitudIngresoProveedorEntrada`: recorta espacios
/// y la placa en blanco queda en `None` (llegó a pie).
#[derive(serde::Deserialize)]
pub struct SolicitudIngresoCorreoEntrada {
    pub cedula: String,
    pub nombre: String,
    pub motivo: String,
    pub placa: Option<String>,
    pub gafete_numero: i64,
}

impl SolicitudIngresoCorreoEntrada {
    pub fn construir(self) -> NuevoIngresoCorreo {
        NuevoIngresoCorreo {
            cedula: self.cedula.trim().to_owned(),
            nombre: self.nombre.trim().to_owned(),
            motivo: self.motivo.trim().to_owned(),
            placa: self
                .placa
                .map(|texto| texto.trim().to_owned())
                .filter(|texto| !texto.is_empty()),
            gafete_numero: self.gafete_numero,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorta_los_textos_y_la_placa_en_blanco_queda_vacia() {
        let datos = SolicitudIngresoCorreoEntrada {
            cedula: " 112345678 ".to_owned(),
            nombre: " Ana Solano ".to_owned(),
            motivo: " Entrevista RH ".to_owned(),
            placa: Some("   ".to_owned()),
            gafete_numero: 5,
        }
        .construir();

        assert_eq!(datos.cedula, "112345678");
        assert_eq!(datos.nombre, "Ana Solano");
        assert_eq!(datos.motivo, "Entrevista RH");
        assert_eq!(datos.placa, None);
    }
}
