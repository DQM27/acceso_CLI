//! Formulario de "cambiar mi propia contraseña" (`/clave`) — Surface
//! enclavada en dos pasos: primero se pide sólo la contraseña actual (gate),
//! y sólo si es correcta aparecen los campos de contraseña nueva/confirmar.
//! A diferencia del formulario de usuario (edición administrativa, donde
//! dejar la contraseña en blanco significa "no cambiarla" y nunca se
//! verifica la anterior), acá el operador está cambiando la suya propia:
//! confirmar identidad primero, antes de pedir la nueva dos veces, es el
//! mismo criterio que `passwd` en cualquier sistema Unix.
//!
//! Todo enmascarado, igual criterio que Password/ConfirmarPassword de
//! `formulario_usuario.rs` — nunca se ve en pantalla, ni siquiera mientras
//! se teclea.

/// Mismo mínimo que exige `usuario_service` (constante privada ahí, no
/// importable) — ver la misma nota en `formulario_usuario.rs`.
const LONGITUD_MINIMA_PASSWORD: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubfasePassword {
    VerificandoActual,
    Cambiando,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampoPassword {
    Nueva,
    Confirmar,
}

#[derive(Debug, Clone)]
pub struct FormularioPassword {
    pub subfase: SubfasePassword,
    pub actual: String,
    pub campo: CampoPassword,
    pub nueva: String,
    pub confirmar: String,
    pub error: Option<String>,
}

impl FormularioPassword {
    pub fn nuevo() -> Self {
        Self {
            subfase: SubfasePassword::VerificandoActual,
            actual: String::new(),
            campo: CampoPassword::Nueva,
            nueva: String::new(),
            confirmar: String::new(),
            error: None,
        }
    }

    pub fn texto_campo(&self) -> &str {
        match self.subfase {
            SubfasePassword::VerificandoActual => &self.actual,
            SubfasePassword::Cambiando => match self.campo {
                CampoPassword::Nueva => &self.nueva,
                CampoPassword::Confirmar => &self.confirmar,
            },
        }
    }

    pub fn asignar_texto(&mut self, texto: &str) {
        match self.subfase {
            SubfasePassword::VerificandoActual => self.actual = texto.to_string(),
            SubfasePassword::Cambiando => match self.campo {
                CampoPassword::Nueva => self.nueva = texto.to_string(),
                CampoPassword::Confirmar => self.confirmar = texto.to_string(),
            },
        }
        self.error = None;
    }

    /// Sólo alterna entre Nueva/Confirmar — no hay nada que mover mientras
    /// se verifica la actual (un solo campo). Devuelve `false` en ese caso
    /// para que el llamador sepa que no hace falta resincronizar el input.
    pub fn alternar_campo(&mut self) -> bool {
        if self.subfase != SubfasePassword::Cambiando {
            return false;
        }
        self.campo = match self.campo {
            CampoPassword::Nueva => CampoPassword::Confirmar,
            CampoPassword::Confirmar => CampoPassword::Nueva,
        };
        true
    }

    /// La actual resultó correcta: abre el segundo paso con los campos en
    /// blanco (nunca se precargan con nada).
    pub fn avanzar_a_cambiar(&mut self) {
        self.subfase = SubfasePassword::Cambiando;
        self.campo = CampoPassword::Nueva;
        self.nueva.clear();
        self.confirmar.clear();
        self.error = None;
    }

    /// La actual resultó incorrecta (o el actor dejó de ser válido): se
    /// limpia y se queda en el primer paso — nunca avanza sin haber pasado
    /// el gate.
    pub fn rechazar_actual(&mut self, mensaje: String) {
        self.actual.clear();
        self.error = Some(mensaje);
    }

    /// Valida longitud/coincidencia de la contraseña nueva — no toca SQLite
    /// (eso lo hace `AppCore::cambiar_mi_password`, que además revuelve a
    /// verificar la actual antes de escribir nada).
    pub fn validar_nueva(&mut self) -> Result<(), String> {
        if self.nueva.chars().count() < LONGITUD_MINIMA_PASSWORD {
            let mensaje = format!("Mínimo {LONGITUD_MINIMA_PASSWORD} caracteres");
            self.campo = CampoPassword::Nueva;
            self.error = Some(mensaje.clone());
            return Err(mensaje);
        }
        if self.nueva != self.confirmar {
            let mensaje = "Las contraseñas no coinciden".to_string();
            self.campo = CampoPassword::Confirmar;
            self.error = Some(mensaje.clone());
            return Err(mensaje);
        }
        self.error = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arranca_pidiendo_la_actual() {
        let form = FormularioPassword::nuevo();
        assert_eq!(form.subfase, SubfasePassword::VerificandoActual);
    }

    #[test]
    fn alternar_campo_no_hace_nada_mientras_se_verifica_la_actual() {
        let mut form = FormularioPassword::nuevo();
        assert!(!form.alternar_campo());
        assert_eq!(form.campo, CampoPassword::Nueva);
    }

    #[test]
    fn avanzar_a_cambiar_limpia_los_campos_nuevos() {
        let mut form = FormularioPassword::nuevo();
        form.nueva = "sobras".to_string();
        form.avanzar_a_cambiar();
        assert_eq!(form.subfase, SubfasePassword::Cambiando);
        assert_eq!(form.nueva, "");
        assert_eq!(form.confirmar, "");
    }

    #[test]
    fn alternar_campo_ciclando_entre_nueva_y_confirmar() {
        let mut form = FormularioPassword::nuevo();
        form.avanzar_a_cambiar();
        assert!(form.alternar_campo());
        assert_eq!(form.campo, CampoPassword::Confirmar);
        assert!(form.alternar_campo());
        assert_eq!(form.campo, CampoPassword::Nueva);
    }

    #[test]
    fn rechazar_actual_limpia_el_campo_y_deja_el_error() {
        let mut form = FormularioPassword::nuevo();
        form.actual = "algo".to_string();
        form.rechazar_actual("Contraseña actual incorrecta".to_string());
        assert_eq!(form.actual, "");
        assert_eq!(form.error.as_deref(), Some("Contraseña actual incorrecta"));
    }

    #[test]
    fn nueva_corta_es_error() {
        let mut form = FormularioPassword::nuevo();
        form.avanzar_a_cambiar();
        form.nueva = "corta".to_string();
        form.confirmar = "corta".to_string();
        assert!(form.validar_nueva().is_err());
        assert_eq!(form.campo, CampoPassword::Nueva);
    }

    #[test]
    fn nueva_y_confirmar_distintas_es_error() {
        let mut form = FormularioPassword::nuevo();
        form.avanzar_a_cambiar();
        form.nueva = "clave1234".to_string();
        form.confirmar = "otraclave".to_string();
        assert!(form.validar_nueva().is_err());
        assert_eq!(form.campo, CampoPassword::Confirmar);
    }

    #[test]
    fn nueva_valida() {
        let mut form = FormularioPassword::nuevo();
        form.avanzar_a_cambiar();
        form.nueva = "clave1234".to_string();
        form.confirmar = "clave1234".to_string();
        assert!(form.validar_nueva().is_ok());
    }
}
