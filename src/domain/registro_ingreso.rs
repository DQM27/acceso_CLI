use chrono::NaiveDateTime;

pub fn salida_es_cronologicamente_valida(
    fecha_hora_ingreso: NaiveDateTime,
    fecha_hora_salida: NaiveDateTime,
) -> bool {
    fecha_hora_salida >= fecha_hora_ingreso
}
