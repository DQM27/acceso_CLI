//! Reglas de criterio del contratista: qué datos son válidos para crearlo o
//! editarlo. Fuente única para el núcleo (`ContratistaService`), el panel
//! web y la Edge Function `admin-crear-contratista` (estos dos vía
//! WebAssembly, `reglas/wasm`).
//!
//! Las reglas que necesitan datos (la empresa existe, la cédula no está
//! repetida) no van acá: las aplica quien tiene los datos, después de
//! [`validar_contratista`].

use chrono::NaiveDate;

use crate::cedula::{Cedula, CedulaInvalida};
use crate::tipo_ingreso::TipoIngreso;

/// Regla de negocio (ver tabla "Reglas para PRAIND y gafete" en
/// `docs/diagramas/diagrama-logico.md`): requiere PRAIND el personal de ruta
/// (sin importar el tipo de ingreso) y, entre los tipos de ingreso, `Praind`
/// e `InHouse`. `PorCorreo` y `Swat` no lo requieren.
pub fn requiere_praind_de(tipo_ingreso: TipoIngreso, personal_ruta: bool) -> bool {
    personal_ruta || matches!(tipo_ingreso, TipoIngreso::Praind | TipoIngreso::InHouse)
}

/// Regla de negocio (misma tabla): el personal de ruta nunca requiere
/// gafete. Entre los tipos de ingreso, sólo `Praind` y `PorCorreo` lo
/// requieren; `InHouse` y `Swat` no.
pub fn requiere_gafete_de(tipo_ingreso: TipoIngreso, personal_ruta: bool) -> bool {
    !personal_ruta && matches!(tipo_ingreso, TipoIngreso::Praind | TipoIngreso::PorCorreo)
}

/// Regla de negocio (pedido del usuario 2026-10-03): `PorCorreo` ya no se
/// elige para un contratista nuevo ni al cambiar el tipo -- esas visitas se
/// registran como ingreso por correo. Sigue existiendo para leer datos
/// viejos (en producción no había ninguno).
pub fn tipo_ingreso_seleccionable(tipo_ingreso: TipoIngreso) -> bool {
    !matches!(tipo_ingreso, TipoIngreso::PorCorreo)
}

/// Regla de negocio (pedido del usuario 2026-09-20): "personal de ruta"
/// sólo existe para `Praind` e `InHouse`. `PorCorreo` y `Swat` no lo
/// admiten.
pub fn admite_personal_ruta(tipo_ingreso: TipoIngreso) -> bool {
    matches!(tipo_ingreso, TipoIngreso::Praind | TipoIngreso::InHouse)
}

/// Regla de negocio: un PRAIND cuya fecha de vencimiento ya pasó (`< hoy`)
/// no habilita a nadie. Vencer HOY todavía cuenta como vigente. Sin fecha
/// no está "vencido" (eso es `PraindRequerido`, otra regla).
pub fn praind_vencido(fecha_vencimiento: Option<NaiveDate>, hoy: NaiveDate) -> bool {
    fecha_vencimiento.is_some_and(|fecha| fecha < hoy)
}

/// Regla de negocio: el nombre sólo tiene letras (con tildes), espacios,
/// apóstrofo y guion -- sin números ni símbolos -- y se guarda en
/// MAYÚSCULAS con los espacios de más colapsados. `None` si no cumple o
/// queda vacío.
pub fn normalizar_nombre(nombre: &str) -> Option<String> {
    let limpio = nombre.split_whitespace().collect::<Vec<_>>().join(" ");
    let valido = !limpio.is_empty()
        && limpio
            .chars()
            .all(|c| c.is_alphabetic() || c == ' ' || c == '\'' || c == '-');
    valido.then(|| limpio.to_uppercase())
}

/// Lo que llega del formulario (escritorio, teléfono o panel), tal cual.
#[derive(Debug, Clone, Copy)]
pub struct DatosContratista<'a> {
    pub cedula: &'a str,
    pub nombre: &'a str,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
}

/// Al editar: lo que el contratista tenía guardado. Decide qué reglas se
/// vuelven a revisar (ver [`validar_contratista`]).
#[derive(Debug, Clone, Copy)]
pub struct EstadoAnterior {
    pub tipo_ingreso: TipoIngreso,
    pub es_personal_ruta: bool,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
}

/// Datos ya validados y normalizados, listos para guardar: cédula en su
/// forma única y nombre en mayúsculas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContratistaValido {
    pub cedula: String,
    pub nombre: String,
    pub tipo_ingreso: TipoIngreso,
    pub fecha_vencimiento_praind: Option<NaiveDate>,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
}

/// Por qué no son válidos los datos. El mismo motivo, con el mismo texto
/// ([`ErrorContratista::mensaje`]) y el mismo código
/// ([`ErrorContratista::codigo`]), en todas las interfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ErrorContratista {
    #[error("La cédula es obligatoria")]
    CedulaVacia,
    #[error("La cédula debe tener sólo números, entre 9 y 13 dígitos")]
    CedulaInvalida,
    #[error("El nombre es obligatorio")]
    NombreVacio,
    #[error("El nombre no puede tener números ni símbolos")]
    NombreInvalido,
    #[error("«Por correo» ya no es un tipo de contratista: registre la visita en «Por correo»")]
    TipoIngresoRetirado,
    #[error("Personal de ruta sólo aplica a PRAIND e IN HOUSE")]
    PersonalRutaNoAdmitido,
    #[error("Fecha PRAIND requerida")]
    PraindRequerido,
    #[error("El PRAIND está vencido — ingrese una fecha vigente")]
    PraindVencido,
}

impl ErrorContratista {
    /// Texto para quien opera. Es el mismo que muestran todas las apps.
    pub fn mensaje(self) -> String {
        self.to_string()
    }

    /// Código estable (no cambia si cambia el texto), para que el panel y
    /// la Edge Function distingan el motivo sin comparar textos.
    pub fn codigo(self) -> &'static str {
        match self {
            Self::CedulaVacia => "cedula_vacia",
            Self::CedulaInvalida => "cedula_invalida",
            Self::NombreVacio => "nombre_vacio",
            Self::NombreInvalido => "nombre_invalido",
            Self::TipoIngresoRetirado => "tipo_ingreso_retirado",
            Self::PersonalRutaNoAdmitido => "personal_ruta_no_admitido",
            Self::PraindRequerido => "praind_requerido",
            Self::PraindVencido => "praind_vencido",
        }
    }
}

/// Todas las reglas de criterio del contratista, en este orden (el primer
/// motivo que falla es el que se informa):
/// 1. cédula obligatoria; sólo nacional o de extranjero (9 a 13 dígitos),
///    guardada en su forma única;
/// 2. nombre obligatorio, sólo letras, guardado en mayúsculas;
/// 3. el tipo se puede elegir (`PorCorreo` está retirado);
/// 4. personal de ruta sólo para PRAIND e IN HOUSE;
/// 5. PRAIND obligatorio si el tipo (o ser personal de ruta) lo requiere y
///    la persona tiene acceso, y vigente.
///
/// Al editar (`anterior` presente), las reglas 3, 4 y "vigente" sólo se
/// revisan si cambió lo que deciden (tipo, la casilla de ruta o la fecha):
/// si no, a alguien con el PRAIND ya vencido no se le podría ni quitar el
/// acceso ni corregir el nombre.
///
/// A quien no tiene acceso no se le pide PRAIND: no va a entrar de todos
/// modos (criterio que antes sólo aplicaba el panel). Al crear desde las
/// apps la persona siempre tiene acceso, así que ahí no cambia nada.
pub fn validar_contratista(
    datos: DatosContratista<'_>,
    anterior: Option<&EstadoAnterior>,
    hoy: NaiveDate,
) -> Result<ContratistaValido, ErrorContratista> {
    let cedula = match Cedula::normalizar(datos.cedula) {
        Ok(cedula) if cedula.es_nacional_o_de_extranjero() => cedula,
        Err(CedulaInvalida::Vacia) => return Err(ErrorContratista::CedulaVacia),
        Ok(_) | Err(_) => return Err(ErrorContratista::CedulaInvalida),
    };

    if datos.nombre.trim().is_empty() {
        return Err(ErrorContratista::NombreVacio);
    }
    let nombre = normalizar_nombre(datos.nombre).ok_or(ErrorContratista::NombreInvalido)?;

    let cambia_tipo = anterior.is_none_or(|previo| previo.tipo_ingreso != datos.tipo_ingreso);
    let cambia_tipo_o_ruta = cambia_tipo
        || anterior.is_none_or(|previo| previo.es_personal_ruta != datos.es_personal_ruta);
    let cambia_praind = cambia_tipo_o_ruta
        || anterior
            .is_none_or(|previo| previo.fecha_vencimiento_praind != datos.fecha_vencimiento_praind);

    if cambia_tipo && !tipo_ingreso_seleccionable(datos.tipo_ingreso) {
        return Err(ErrorContratista::TipoIngresoRetirado);
    }
    if cambia_tipo_o_ruta && datos.es_personal_ruta && !admite_personal_ruta(datos.tipo_ingreso) {
        return Err(ErrorContratista::PersonalRutaNoAdmitido);
    }

    if datos.tiene_acceso && requiere_praind_de(datos.tipo_ingreso, datos.es_personal_ruta) {
        if datos.fecha_vencimiento_praind.is_none() {
            return Err(ErrorContratista::PraindRequerido);
        }
        if cambia_praind && praind_vencido(datos.fecha_vencimiento_praind, hoy) {
            return Err(ErrorContratista::PraindVencido);
        }
    }

    Ok(ContratistaValido {
        cedula: cedula.into_string(),
        nombre,
        tipo_ingreso: datos.tipo_ingreso,
        fecha_vencimiento_praind: datos.fecha_vencimiento_praind,
        es_personal_ruta: datos.es_personal_ruta,
        tiene_acceso: datos.tiene_acceso,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hoy() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 4).unwrap()
    }

    fn datos(tipo_ingreso: TipoIngreso, praind: Option<NaiveDate>) -> DatosContratista<'static> {
        DatosContratista {
            cedula: "1-1111-1111",
            nombre: "  ana  solano ",
            tipo_ingreso,
            fecha_vencimiento_praind: praind,
            es_personal_ruta: false,
            tiene_acceso: true,
        }
    }

    #[test]
    fn requiere_praind_de_personal_de_ruta_sin_importar_el_tipo() {
        assert!(requiere_praind_de(TipoIngreso::Swat, true));
        assert!(requiere_praind_de(TipoIngreso::PorCorreo, true));
    }

    #[test]
    fn requiere_praind_de_segun_tipo_de_ingreso_sin_personal_de_ruta() {
        assert!(requiere_praind_de(TipoIngreso::Praind, false));
        assert!(requiere_praind_de(TipoIngreso::InHouse, false));
        assert!(!requiere_praind_de(TipoIngreso::PorCorreo, false));
        assert!(!requiere_praind_de(TipoIngreso::Swat, false));
    }

    #[test]
    fn requiere_gafete_de_nunca_para_personal_de_ruta() {
        assert!(!requiere_gafete_de(TipoIngreso::Praind, true));
        assert!(!requiere_gafete_de(TipoIngreso::PorCorreo, true));
    }

    #[test]
    fn requiere_gafete_de_segun_tipo_de_ingreso_sin_personal_de_ruta() {
        assert!(requiere_gafete_de(TipoIngreso::Praind, false));
        assert!(requiere_gafete_de(TipoIngreso::PorCorreo, false));
        assert!(!requiere_gafete_de(TipoIngreso::InHouse, false));
        assert!(!requiere_gafete_de(TipoIngreso::Swat, false));
    }

    #[test]
    fn admite_personal_ruta_solo_praind_e_in_house() {
        assert!(admite_personal_ruta(TipoIngreso::Praind));
        assert!(admite_personal_ruta(TipoIngreso::InHouse));
        assert!(!admite_personal_ruta(TipoIngreso::PorCorreo));
        assert!(!admite_personal_ruta(TipoIngreso::Swat));
    }

    #[test]
    fn praind_vencido_solo_si_la_fecha_ya_paso() {
        let hoy = NaiveDate::from_ymd_opt(2026, 9, 27).unwrap();
        assert!(praind_vencido(NaiveDate::from_ymd_opt(2026, 9, 26), hoy));
        assert!(!praind_vencido(Some(hoy), hoy));
        assert!(!praind_vencido(NaiveDate::from_ymd_opt(2027, 1, 1), hoy));
        assert!(!praind_vencido(None, hoy));
    }

    #[test]
    fn normalizar_nombre_mayusculas_sin_numeros_ni_simbolos() {
        assert_eq!(
            normalizar_nombre("  maría  josé o'neil-rojas "),
            Some("MARÍA JOSÉ O'NEIL-ROJAS".to_string())
        );
        assert_eq!(normalizar_nombre("Ana 2"), None);
        assert_eq!(normalizar_nombre("Ana@"), None);
        assert_eq!(normalizar_nombre("   "), None);
    }

    #[test]
    fn valida_y_normaliza_cedula_y_nombre() {
        let valido = validar_contratista(datos(TipoIngreso::Swat, None), None, hoy()).unwrap();
        assert_eq!(valido.cedula, "111111111");
        assert_eq!(valido.nombre, "ANA SOLANO");
        assert!(valido.tiene_acceso);
    }

    #[test]
    fn informa_el_primer_motivo_en_orden() {
        let mut sin_cedula = datos(TipoIngreso::PorCorreo, None);
        sin_cedula.cedula = " ";
        sin_cedula.nombre = "";
        assert_eq!(
            validar_contratista(sin_cedula, None, hoy()),
            Err(ErrorContratista::CedulaVacia)
        );

        let mut pasaporte = datos(TipoIngreso::Swat, None);
        pasaporte.cedula = "AB123456";
        assert_eq!(
            validar_contratista(pasaporte, None, hoy()),
            Err(ErrorContratista::CedulaInvalida)
        );

        let mut sin_nombre = datos(TipoIngreso::Swat, None);
        sin_nombre.nombre = "  ";
        assert_eq!(
            validar_contratista(sin_nombre, None, hoy()),
            Err(ErrorContratista::NombreVacio)
        );

        let mut nombre_raro = datos(TipoIngreso::Swat, None);
        nombre_raro.nombre = "Ana 2";
        assert_eq!(
            validar_contratista(nombre_raro, None, hoy()),
            Err(ErrorContratista::NombreInvalido)
        );
    }

    #[test]
    fn por_correo_esta_retirado_al_crear_y_al_cambiar_pero_no_si_ya_lo_era() {
        assert_eq!(
            validar_contratista(datos(TipoIngreso::PorCorreo, None), None, hoy()),
            Err(ErrorContratista::TipoIngresoRetirado)
        );
        let ya_era = EstadoAnterior {
            tipo_ingreso: TipoIngreso::PorCorreo,
            es_personal_ruta: false,
            fecha_vencimiento_praind: None,
        };
        assert!(
            validar_contratista(datos(TipoIngreso::PorCorreo, None), Some(&ya_era), hoy()).is_ok()
        );
    }

    #[test]
    fn personal_de_ruta_solo_para_praind_e_in_house() {
        let mut swat_de_ruta = datos(TipoIngreso::Swat, None);
        swat_de_ruta.es_personal_ruta = true;
        assert_eq!(
            validar_contratista(swat_de_ruta, None, hoy()),
            Err(ErrorContratista::PersonalRutaNoAdmitido)
        );
    }

    #[test]
    fn praind_requerido_y_vigente_si_tiene_acceso() {
        assert_eq!(
            validar_contratista(datos(TipoIngreso::Praind, None), None, hoy()),
            Err(ErrorContratista::PraindRequerido)
        );
        let ayer = NaiveDate::from_ymd_opt(2026, 10, 3);
        assert_eq!(
            validar_contratista(datos(TipoIngreso::InHouse, ayer), None, hoy()),
            Err(ErrorContratista::PraindVencido)
        );
        assert!(validar_contratista(datos(TipoIngreso::Praind, Some(hoy())), None, hoy()).is_ok());
    }

    #[test]
    fn sin_acceso_no_se_pide_praind() {
        let mut bloqueado = datos(TipoIngreso::Praind, None);
        bloqueado.tiene_acceso = false;
        assert!(validar_contratista(bloqueado, None, hoy()).is_ok());
    }

    #[test]
    fn al_editar_sin_cambiar_tipo_ni_fecha_no_se_revisa_el_vencimiento() {
        let vencido = NaiveDate::from_ymd_opt(2026, 1, 1);
        let anterior = EstadoAnterior {
            tipo_ingreso: TipoIngreso::Praind,
            es_personal_ruta: false,
            fecha_vencimiento_praind: vencido,
        };
        assert!(
            validar_contratista(datos(TipoIngreso::Praind, vencido), Some(&anterior), hoy())
                .is_ok()
        );
    }

    #[test]
    fn cada_motivo_tiene_codigo_y_mensaje() {
        assert_eq!(ErrorContratista::PraindVencido.codigo(), "praind_vencido");
        assert_eq!(
            ErrorContratista::CedulaInvalida.mensaje(),
            "La cédula debe tener sólo números, entre 9 y 13 dígitos"
        );
    }
}
