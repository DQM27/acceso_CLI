use chrono::{DateTime, Utc};

pub fn salida_es_cronologicamente_valida(
    fecha_hora_ingreso: DateTime<Utc>,
    fecha_hora_salida: DateTime<Utc>,
) -> bool {
    fecha_hora_salida >= fecha_hora_ingreso
}
