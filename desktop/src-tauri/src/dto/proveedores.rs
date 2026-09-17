/// Espejo de los argumentos de `AppCore::registrar_ingreso_proveedor` --
/// mismo criterio que `SolicitudSalidaRutaEntrada`: recorta espacios y
/// convierte placa en blanco a `None` (la placa es opcional, sin catálogo
/// de vehículos, ver `docs/features-futuras/plan-control-proveedores.md`).
#[derive(serde::Deserialize)]
pub struct SolicitudIngresoProveedorEntrada {
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: i64,
    pub placa: Option<String>,
    pub gafete_numero: i64,
}

pub struct DatosIngresoProveedor {
    pub cedula: String,
    pub nombre: String,
    pub empresa_id: i64,
    pub placa: Option<String>,
    pub gafete_numero: i64,
}

impl SolicitudIngresoProveedorEntrada {
    pub fn construir(self) -> DatosIngresoProveedor {
        DatosIngresoProveedor {
            cedula: self.cedula.trim().to_owned(),
            nombre: self.nombre.trim().to_owned(),
            empresa_id: self.empresa_id,
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
    fn la_placa_en_blanco_se_convierte_en_ninguna() {
        let datos = SolicitudIngresoProveedorEntrada {
            cedula: " 1-1111 ".to_owned(),
            nombre: " Juan Perez ".to_owned(),
            empresa_id: 3,
            placa: Some("   ".to_owned()),
            gafete_numero: 7,
        }
        .construir();

        assert_eq!(datos.cedula, "1-1111");
        assert_eq!(datos.nombre, "Juan Perez");
        assert_eq!(datos.placa, None);
    }

    #[test]
    fn la_placa_con_contenido_se_recorta() {
        let datos = SolicitudIngresoProveedorEntrada {
            cedula: "1-1111".to_owned(),
            nombre: "Juan Perez".to_owned(),
            empresa_id: 3,
            placa: Some("  C12345  ".to_owned()),
            gafete_numero: 7,
        }
        .construir();

        assert_eq!(datos.placa, Some("C12345".to_owned()));
    }
}
