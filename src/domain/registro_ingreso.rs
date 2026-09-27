use chrono::{DateTime, Utc};

use crate::models::medio_ingreso::MedioIngreso;

pub fn salida_es_cronologicamente_valida(
    fecha_hora_ingreso: DateTime<Utc>,
    fecha_hora_salida: DateTime<Utc>,
) -> bool {
    fecha_hora_salida >= fecha_hora_ingreso
}

/// Regla de negocio: la placa sólo existe cuando el ingreso es en
/// vehículo. Lo que se haya tipeado con "Caminando" se descarta (quien
/// opera pudo escribir algo antes de cambiar de medio) y los espacios de
/// los bordes no cuentan. Una placa vacía en vehículo queda `None` y la
/// rechaza `RegistroIngresoService` (`PlacaRequerida`).
///
/// La usa el puente móvil antes de registrar; el escritorio hace lo mismo
/// en TypeScript (`NuevoIngresoModal.tsx`), ver
/// `docs/auditorias/reglas-duplicadas-escritorio-2026-09-27.md`.
pub fn placa_segun_medio(medio: MedioIngreso, placa: Option<String>) -> Option<String> {
    match medio {
        MedioIngreso::Vehiculo => placa
            .map(|texto| texto.trim().to_string())
            .filter(|texto| !texto.is_empty()),
        MedioIngreso::Caminando => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placa_segun_medio_solo_en_vehiculo_y_sin_espacios() {
        assert_eq!(
            placa_segun_medio(MedioIngreso::Vehiculo, Some(" C123 ".into())),
            Some("C123".into())
        );
        assert_eq!(
            placa_segun_medio(MedioIngreso::Vehiculo, Some("   ".into())),
            None
        );
        assert_eq!(
            placa_segun_medio(MedioIngreso::Caminando, Some("C123".into())),
            None
        );
    }
}
