//! Formulario de alta y edición de contratistas para la interfaz `--cli`.
//!
//! Lógica pura: sin terminal, sin `AppCore`, sin SQLite. El estado vive entero
//! en [`FormularioContratista`] y cada tecla se traduce a un método; el render
//! y `mod.rs` sólo lo leen. Así toda la navegación, la fecha con `/`
//! automáticos y la validación se prueban sin levantar una terminal.
//!
//! Criterios replicados de la TUI clásica (que es intocable): campos y orden
//! del formulario de contratistas, defaults de alta (tipo PRAIND, sin ruta,
//! acceso concedido), fecha `DD/MM/YYYY` con `/` auto-insertados, y campos
//! bloqueados según permisos del rol (cédula y acceso).

use chrono::NaiveDate;

use crate::database::queries::contratistas::ContratistaResumen;
use crate::models::empresa::Empresa;
use crate::models::tipo_ingreso::TipoIngreso;
use crate::services::contratista_service::{DatosActualizacionContratista, DatosContratista};
use crate::texto::plegar_para_busqueda;

/// Largos máximos de los campos de texto — los mismos del formulario de la
/// TUI clásica.
pub const MAX_CEDULA: usize = 30;
pub const MAX_NOMBRE: usize = 60;
/// Dígitos de la fecha (sin contar las `/` que se insertan solas).
const MAX_DIGITOS_FECHA: usize = 8;

/// Empresas visibles a la vez en el selector — el render y el manejo de
/// teclas comparten este tope para que la selección nunca quede fuera.
pub const MAX_VISIBLES_EMPRESAS: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModoFormulario {
    Nuevo,
    Editar { id: i64 },
}

/// Campos en el orden en que se recorren con ↑↓. No hay un campo
/// "Confirmar": Enter intenta guardar desde cualquiera de éstos (igual que
/// la TUI clásica) — un campo-botón al final obligaba a navegar hasta él
/// para poder confirmar, rompiendo la regla de que Enter significa lo mismo
/// en toda la interfaz (§2/§5.2, DEC-025).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Campo {
    Cedula,
    Nombre,
    Empresa,
    Tipo,
    FechaPraind,
    Ruta,
    Acceso,
}

impl Campo {
    pub const ORDEN: [Self; 7] = [
        Self::Cedula,
        Self::Nombre,
        Self::Empresa,
        Self::Tipo,
        Self::FechaPraind,
        Self::Ruta,
        Self::Acceso,
    ];

    pub fn etiqueta(self) -> &'static str {
        match self {
            Self::Cedula => "Cédula",
            Self::Nombre => "Nombre",
            Self::Empresa => "Empresa",
            Self::Tipo => "Tipo",
            Self::FechaPraind => "Fecha PRAIND",
            Self::Ruta => "Personal de ruta",
            Self::Acceso => "Acceso",
        }
    }

    /// Los campos de texto se editan tecleando en el input; el resto se
    /// modifican con Space/←/→ (empresa abre el selector, tipo cicla,
    /// ruta/acceso conmutan).
    pub fn es_texto(self) -> bool {
        matches!(self, Self::Cedula | Self::Nombre | Self::FechaPraind)
    }

    /// Campos que pueden quedar vacíos o inválidos — los únicos que muestran
    /// ✓/× de estado en el render (Tipo/Ruta/Acceso siempre tienen un valor
    /// por defecto, un check ahí no aportaría información).
    pub fn admite_estado(self) -> bool {
        matches!(
            self,
            Self::Cedula | Self::Nombre | Self::Empresa | Self::FechaPraind
        )
    }
}

/// Qué está pasando dentro del formulario: edición normal de campos, el
/// selector de empresa (con su propia lista y selección) o la tarjeta de
/// resumen previa a persistir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subfase {
    Editando,
    EligiendoEmpresa { seleccion: usize },
    Resumen,
}

#[derive(Debug, Clone)]
pub struct FormularioContratista {
    pub modo: ModoFormulario,
    pub campo: Campo,
    pub subfase: Subfase,
    pub cedula: String,
    pub nombre: String,
    /// Empresa elegida: `(id, nombre)` — el nombre viaja para no tener que
    /// resolverlo en cada render.
    pub empresa: Option<(i64, String)>,
    pub tipo: TipoIngreso,
    /// Texto tal como lo teclea el operador, con las `/` ya insertadas.
    pub fecha_praind: String,
    pub es_personal_ruta: bool,
    pub tiene_acceso: bool,
    /// Catálogo cargado al abrir el formulario.
    pub empresas: Vec<Empresa>,
    /// Permisos del actor: replican lo que la TUI hace con `cedula_editable`.
    pub cedula_editable: bool,
    pub acceso_editable: bool,
    /// Errores del último intento de confirmación, por campo.
    pub errores: Vec<(Campo, String)>,
}

/// Tupla intermedia de `valores`: los campos ya validados, antes de vestirlos
/// como `DatosContratista` o `DatosActualizacionContratista`.
type ValoresValidados = (
    String,
    String,
    i64,
    TipoIngreso,
    Option<NaiveDate>,
    bool,
    bool,
);

impl FormularioContratista {
    /// Alta con los defaults de la TUI: tipo PRAIND, no es personal de ruta,
    /// acceso concedido. La cédula siempre es editable al crear.
    pub fn nuevo(empresas: Vec<Empresa>, acceso_editable: bool) -> Self {
        Self {
            modo: ModoFormulario::Nuevo,
            campo: Campo::Cedula,
            subfase: Subfase::Editando,
            cedula: String::new(),
            nombre: String::new(),
            empresa: None,
            tipo: TipoIngreso::Praind,
            fecha_praind: String::new(),
            es_personal_ruta: false,
            tiene_acceso: true,
            empresas,
            cedula_editable: true,
            acceso_editable,
            errores: Vec::new(),
        }
    }

    /// Edición precargada desde el resumen de la búsqueda (que ya trae todos
    /// los campos del formulario). Si la cédula no es editable por permisos,
    /// el campo inicial es Nombre — igual que la TUI.
    pub fn editar(
        resumen: &ContratistaResumen,
        empresas: Vec<Empresa>,
        cedula_editable: bool,
        acceso_editable: bool,
    ) -> Self {
        Self {
            modo: ModoFormulario::Editar { id: resumen.id },
            campo: if cedula_editable {
                Campo::Cedula
            } else {
                Campo::Nombre
            },
            subfase: Subfase::Editando,
            cedula: resumen.cedula.clone(),
            nombre: resumen.nombre.clone(),
            empresa: Some((resumen.empresa_id, resumen.empresa_nombre.clone())),
            tipo: resumen.tipo_ingreso,
            fecha_praind: resumen
                .fecha_vencimiento_praind
                .map(|fecha| fecha.format("%d/%m/%Y").to_string())
                .unwrap_or_default(),
            es_personal_ruta: resumen.es_personal_ruta,
            tiene_acceso: resumen.tiene_acceso,
            empresas,
            cedula_editable,
            acceso_editable,
            errores: Vec::new(),
        }
    }

    /// Un campo bloqueado por permisos se muestra apagado y se salta en la
    /// navegación; su valor viaja intacto al guardar.
    pub fn campo_habilitado(&self, campo: Campo) -> bool {
        match campo {
            Campo::Cedula => matches!(self.modo, ModoFormulario::Nuevo) || self.cedula_editable,
            Campo::Acceso => self.acceso_editable,
            _ => true,
        }
    }

    fn campos_navegables(&self) -> Vec<Campo> {
        Campo::ORDEN
            .into_iter()
            .filter(|campo| self.campo_habilitado(*campo))
            .collect()
    }

    /// Mueve el campo activo `delta` posiciones dentro de los navegables,
    /// sin salirse de los extremos. Devuelve `true` si el campo cambió (el
    /// llamador sincroniza el input con el nuevo campo).
    pub fn mover_campo(&mut self, delta: isize) -> bool {
        let navegables = self.campos_navegables();
        let Some(indice) = navegables.iter().position(|campo| *campo == self.campo) else {
            return false;
        };
        let nuevo = (indice as isize + delta).clamp(0, navegables.len() as isize - 1) as usize;
        let cambio = navegables[nuevo] != self.campo;
        self.campo = navegables[nuevo];
        cambio
    }

    /// Space/←/→ sobre un campo no textual: cicla el tipo de ingreso o
    /// conmuta los booleanos. Sin efecto en campos de texto.
    pub fn alternar(&mut self) {
        match self.campo {
            Campo::Tipo => {
                let indice = TipoIngreso::ALL
                    .iter()
                    .position(|tipo| *tipo == self.tipo)
                    .unwrap_or(0);
                self.tipo = TipoIngreso::ALL[(indice + 1) % TipoIngreso::ALL.len()];
            }
            Campo::Ruta => self.es_personal_ruta = !self.es_personal_ruta,
            Campo::Acceso if self.acceso_editable => self.tiene_acceso = !self.tiene_acceso,
            _ => {}
        }
    }

    /// Texto del campo activo para sincronizar el input. `None` en campos no
    /// textuales (el input queda vacío y sin efecto sobre ellos).
    pub fn texto_campo(&self) -> Option<&str> {
        match self.campo {
            Campo::Cedula => Some(&self.cedula),
            Campo::Nombre => Some(&self.nombre),
            Campo::FechaPraind => Some(&self.fecha_praind),
            _ => None,
        }
    }

    /// Vuelca el contenido del input sobre el campo activo, saneando según el
    /// campo: cédula sólo dígitos, nombre sólo letras/espacios (con acentos
    /// y ñ) más guion y apóstrofo para nombres compuestos, fecha con sólo
    /// dígitos y `/` auto-insertadas — todos con su largo máximo. Un
    /// carácter que no corresponde no se inserta y punto, igual criterio
    /// que ya tenía la fecha (nunca fue una regla nueva, sólo faltaba
    /// aplicarla a los otros dos campos de texto).
    pub fn asignar_texto(&mut self, texto: &str) {
        match self.campo {
            Campo::Cedula => {
                self.cedula = texto
                    .chars()
                    .filter(char::is_ascii_digit)
                    .take(MAX_CEDULA)
                    .collect();
            }
            Campo::Nombre => {
                self.nombre = texto
                    .chars()
                    .filter(|c| c.is_alphabetic() || c.is_whitespace() || *c == '-' || *c == '\'')
                    .take(MAX_NOMBRE)
                    .collect();
            }
            Campo::FechaPraind => {
                self.fecha_praind = formatear_fecha(texto);
            }
            _ => {}
        }
        // Corregir un campo limpia su error de la última validación.
        self.errores.retain(|(campo, _)| *campo != self.campo);
    }

    /// Misma regla de negocio que `Contratista::requiere_praind`: ruta o
    /// (Praind ∨ InHouse).
    pub fn requiere_praind(&self) -> bool {
        self.es_personal_ruta || matches!(self.tipo, TipoIngreso::Praind | TipoIngreso::InHouse)
    }

    /// Empresas elegibles filtradas por el texto del selector: sólo activas,
    /// más la empresa actual aunque esté inactiva (si no, al editar no se
    /// podría conservar). La comparación pliega tildes y mayúsculas.
    pub fn empresas_filtradas(&self, consulta: &str) -> Vec<&Empresa> {
        let consulta = plegar_para_busqueda(consulta.trim());
        let empresa_actual = self.empresa.as_ref().map(|(id, _)| *id);
        self.empresas
            .iter()
            .filter(|empresa| empresa.activo || Some(empresa.id) == empresa_actual)
            .filter(|empresa| {
                consulta.is_empty() || plegar_para_busqueda(&empresa.nombre).contains(&consulta)
            })
            .collect()
    }

    pub fn error_de(&self, campo: Campo) -> Option<&str> {
        self.errores
            .iter()
            .find(|(c, _)| *c == campo)
            .map(|(_, mensaje)| mensaje.as_str())
    }

    /// Valida todo el formulario. Con errores los deja en `self.errores` y
    /// devuelve `Err`; sin errores devuelve los datos listos para persistir.
    pub fn validar(&mut self) -> Result<DatosContratista, Vec<(Campo, String)>> {
        match self.valores() {
            Ok((cedula, nombre, empresa_id, tipo, fecha, ruta, acceso)) => {
                self.errores.clear();
                Ok(DatosContratista {
                    cedula,
                    nombre,
                    empresa_id,
                    tipo_ingreso: tipo,
                    fecha_vencimiento_praind: fecha,
                    es_personal_ruta: ruta,
                    tiene_acceso: acceso,
                })
            }
            Err(errores) => {
                self.errores = errores.clone();
                Err(errores)
            }
        }
    }

    /// Los mismos datos validados pero para actualizar. Llama a `validar`
    /// antes implícitamente: si hay errores quedan registrados igual.
    pub fn datos_actualizacion(
        &mut self,
    ) -> Result<DatosActualizacionContratista, Vec<(Campo, String)>> {
        let datos = self.validar()?;
        Ok(DatosActualizacionContratista {
            cedula: datos.cedula,
            nombre: datos.nombre,
            empresa_id: datos.empresa_id,
            tipo_ingreso: datos.tipo_ingreso,
            fecha_vencimiento_praind: datos.fecha_vencimiento_praind,
            es_personal_ruta: datos.es_personal_ruta,
            tiene_acceso: datos.tiene_acceso,
        })
    }

    /// Validación compartida por alta y edición. La fecha sólo se exige y se
    /// parsea cuando el contratista requiere PRAIND — igual que el formulario
    /// de la TUI clásica; si no aplica, se guarda `None`.
    fn valores(&self) -> Result<ValoresValidados, Vec<(Campo, String)>> {
        let mut errores = Vec::new();

        let cedula = self.cedula.trim().to_string();
        if cedula.is_empty() {
            errores.push((Campo::Cedula, "Escriba la cédula".to_string()));
        }
        let nombre = self.nombre.trim().to_string();
        if nombre.is_empty() {
            errores.push((Campo::Nombre, "Escriba el nombre".to_string()));
        }
        let empresa_id = match &self.empresa {
            Some((id, _)) => Some(*id),
            None => {
                errores.push((Campo::Empresa, "Elija una empresa".to_string()));
                None
            }
        };

        let fecha = if self.requiere_praind() {
            let texto = self.fecha_praind.trim();
            if texto.is_empty() {
                errores.push((Campo::FechaPraind, "Fecha PRAIND requerida".to_string()));
                None
            } else {
                match NaiveDate::parse_from_str(texto, "%d/%m/%Y") {
                    Ok(fecha) => Some(fecha),
                    Err(_) => {
                        errores.push((
                            Campo::FechaPraind,
                            "Fecha inválida. Use DD/MM/YYYY".to_string(),
                        ));
                        None
                    }
                }
            }
        } else {
            None
        };

        if errores.is_empty() {
            Ok((
                cedula,
                nombre,
                empresa_id.expect("empresa presente cuando no hay errores"),
                self.tipo,
                fecha,
                self.es_personal_ruta,
                self.tiene_acceso,
            ))
        } else {
            Err(errores)
        }
    }
}

/// Sólo dígitos (máx. 8) con `/` tras el día y el mes: "22082026" →
/// "22/08/2026", igual que el `agregar_fecha` del formulario clásico.
fn formatear_fecha(texto: &str) -> String {
    let digitos: String = texto
        .chars()
        .filter(|c| c.is_ascii_digit())
        .take(MAX_DIGITOS_FECHA)
        .collect();
    let mut fecha = String::new();
    for (indice, digito) in digitos.chars().enumerate() {
        if indice == 2 || indice == 4 {
            fecha.push('/');
        }
        fecha.push(digito);
    }
    fecha
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empresas() -> Vec<Empresa> {
        vec![
            Empresa {
                id: 1,
                nombre: "Constructora Pérez".to_string(),
                activo: true,
            },
            Empresa {
                id: 2,
                nombre: "Eléctrica Quesada".to_string(),
                activo: true,
            },
            Empresa {
                id: 3,
                nombre: "Inactiva S.A.".to_string(),
                activo: false,
            },
        ]
    }

    fn resumen() -> ContratistaResumen {
        ContratistaResumen {
            id: 7,
            empresa_id: 2,
            cedula: "119430546".to_string(),
            nombre: "Carlos Pérez".to_string(),
            empresa_nombre: "Eléctrica Quesada".to_string(),
            tipo_ingreso: TipoIngreso::InHouse,
            fecha_vencimiento_praind: NaiveDate::from_ymd_opt(2027, 3, 15),
            es_personal_ruta: false,
            tiene_acceso: true,
            tiene_ingreso_activo: false,
        }
    }

    fn formulario_valido() -> FormularioContratista {
        let mut form = FormularioContratista::nuevo(empresas(), true);
        form.cedula = "119430546".to_string();
        form.nombre = "Carlos Pérez".to_string();
        form.empresa = Some((1, "Constructora Pérez".to_string()));
        form.fecha_praind = "15/03/2027".to_string();
        form
    }

    // ── Navegación y permisos ────────────────────────────────────────────

    #[test]
    fn navegacion_recorre_todos_los_campos_en_orden() {
        let mut form = FormularioContratista::nuevo(empresas(), true);
        assert_eq!(form.campo, Campo::Cedula);
        let esperado = [
            Campo::Nombre,
            Campo::Empresa,
            Campo::Tipo,
            Campo::FechaPraind,
            Campo::Ruta,
            Campo::Acceso,
        ];
        for campo in esperado {
            assert!(form.mover_campo(1));
            assert_eq!(form.campo, campo);
        }
        // En el último no se sale del extremo.
        assert!(!form.mover_campo(1));
        assert_eq!(form.campo, Campo::Acceso);
    }

    #[test]
    fn navegacion_salta_campos_bloqueados_por_permiso() {
        let mut form = FormularioContratista::editar(&resumen(), empresas(), false, false);
        // Sin permiso de cédula: arranca en Nombre.
        assert_eq!(form.campo, Campo::Nombre);
        // Sin permiso de acceso: Ruta ya es el último campo navegable.
        form.campo = Campo::Ruta;
        assert!(!form.mover_campo(1));
        assert_eq!(form.campo, Campo::Ruta);
        // Hacia atrás sigue funcionando con normalidad.
        assert!(form.mover_campo(-1));
        assert_eq!(form.campo, Campo::FechaPraind);
    }

    #[test]
    fn edicion_con_permiso_arranca_en_cedula() {
        let form = FormularioContratista::editar(&resumen(), empresas(), true, true);
        assert_eq!(form.campo, Campo::Cedula);
    }

    // ── Alternar valores ─────────────────────────────────────────────────

    #[test]
    fn alternar_cicla_el_tipo_de_ingreso() {
        let mut form = FormularioContratista::nuevo(empresas(), true);
        form.campo = Campo::Tipo;
        assert_eq!(form.tipo, TipoIngreso::Praind);
        form.alternar();
        assert_eq!(form.tipo, TipoIngreso::InHouse);
        form.alternar();
        assert_eq!(form.tipo, TipoIngreso::PorCorreo);
        form.alternar();
        assert_eq!(form.tipo, TipoIngreso::Swat);
        form.alternar();
        assert_eq!(form.tipo, TipoIngreso::Praind);
    }

    #[test]
    fn alternar_conmuta_booleanos_solo_con_permiso() {
        let mut form = FormularioContratista::nuevo(empresas(), false);
        form.campo = Campo::Ruta;
        form.alternar();
        assert!(form.es_personal_ruta);
        form.campo = Campo::Acceso;
        form.alternar();
        // Sin permiso de acceso el valor se conserva.
        assert!(form.tiene_acceso);
    }

    // ── Texto de campos ──────────────────────────────────────────────────

    #[test]
    fn fecha_inserta_las_barras_y_limita_digitos() {
        let mut form = FormularioContratista::nuevo(empresas(), true);
        form.campo = Campo::FechaPraind;
        form.asignar_texto("2203");
        assert_eq!(form.fecha_praind, "22/03");
        form.asignar_texto("22032027");
        assert_eq!(form.fecha_praind, "22/03/2027");
        form.asignar_texto("22/03/20271234");
        assert_eq!(form.fecha_praind, "22/03/2027");
        // Letras y símbolos se descartan.
        form.asignar_texto("1a5/mar/27");
        assert_eq!(form.fecha_praind, "15/27");
    }

    #[test]
    fn texto_respeta_largos_maximos() {
        let mut form = FormularioContratista::nuevo(empresas(), true);
        form.campo = Campo::Cedula;
        form.asignar_texto(&"1".repeat(40));
        assert_eq!(form.cedula.chars().count(), MAX_CEDULA);
        form.campo = Campo::Nombre;
        form.asignar_texto(&"a".repeat(70));
        assert_eq!(form.nombre.chars().count(), MAX_NOMBRE);
    }

    #[test]
    fn cedula_solo_admite_digitos() {
        let mut form = FormularioContratista::nuevo(empresas(), true);
        form.campo = Campo::Cedula;
        form.asignar_texto("1a1b9c4-3 0546");
        assert_eq!(form.cedula, "119430546");
    }

    #[test]
    fn nombre_admite_letras_acentos_ene_espacios_guion_y_apostrofo() {
        let mut form = FormularioContratista::nuevo(empresas(), true);
        form.campo = Campo::Nombre;
        form.asignar_texto("María José Peña-O'Brien");
        assert_eq!(form.nombre, "María José Peña-O'Brien");
    }

    #[test]
    fn nombre_descarta_digitos_y_simbolos() {
        let mut form = FormularioContratista::nuevo(empresas(), true);
        form.campo = Campo::Nombre;
        form.asignar_texto("Carlos123 #Pérez!");
        assert_eq!(form.nombre, "Carlos Pérez");
    }

    #[test]
    fn escribir_limpia_el_error_del_campo() {
        let mut form = formulario_valido();
        form.cedula.clear();
        assert!(form.validar().is_err());
        assert!(form.error_de(Campo::Cedula).is_some());
        form.campo = Campo::Cedula;
        form.asignar_texto("123");
        assert!(form.error_de(Campo::Cedula).is_none());
    }

    // ── Selector de empresa ──────────────────────────────────────────────

    #[test]
    fn empresas_filtradas_excluye_inactivas_y_pliega_tildes() {
        let form = FormularioContratista::nuevo(empresas(), true);
        let todas = form.empresas_filtradas("");
        assert_eq!(todas.len(), 2);
        let filtradas = form.empresas_filtradas("electrica");
        assert_eq!(filtradas.len(), 1);
        assert_eq!(filtradas[0].id, 2);
    }

    #[test]
    fn empresas_filtradas_conserva_la_actual_aunque_inactiva() {
        let mut form = FormularioContratista::nuevo(empresas(), true);
        form.empresa = Some((3, "Inactiva S.A.".to_string()));
        let todas = form.empresas_filtradas("");
        assert_eq!(todas.len(), 3);
    }

    // ── Validación ───────────────────────────────────────────────────────

    #[test]
    fn validacion_completa_devuelve_datos() {
        let mut form = formulario_valido();
        let datos = form.validar().expect("formulario válido");
        assert_eq!(datos.cedula, "119430546");
        assert_eq!(datos.nombre, "Carlos Pérez");
        assert_eq!(datos.empresa_id, 1);
        assert_eq!(datos.tipo_ingreso, TipoIngreso::Praind);
        assert_eq!(
            datos.fecha_vencimiento_praind,
            NaiveDate::from_ymd_opt(2027, 3, 15)
        );
        assert!(!datos.es_personal_ruta);
        assert!(datos.tiene_acceso);
    }

    #[test]
    fn validacion_marca_campos_vacios_y_sin_empresa() {
        let mut form = FormularioContratista::nuevo(empresas(), true);
        let errores = form.validar().err().expect("vacío no valida");
        let campos: Vec<Campo> = errores.iter().map(|(campo, _)| *campo).collect();
        assert!(campos.contains(&Campo::Cedula));
        assert!(campos.contains(&Campo::Nombre));
        assert!(campos.contains(&Campo::Empresa));
        assert!(campos.contains(&Campo::FechaPraind)); // Praind por defecto la requiere
    }

    #[test]
    fn fecha_invalida_es_error_de_validacion() {
        let mut form = formulario_valido();
        form.fecha_praind = "31/02/2027".to_string();
        let errores = form.validar().err().expect("fecha imposible");
        assert_eq!(errores.len(), 1);
        assert_eq!(errores[0].0, Campo::FechaPraind);
        assert!(errores[0].1.contains("DD/MM/YYYY"));
    }

    #[test]
    fn tipo_sin_praind_no_exige_fecha_y_la_descarta() {
        let mut form = formulario_valido();
        form.tipo = TipoIngreso::Swat;
        form.fecha_praind.clear();
        let datos = form.validar().expect("SWAT no requiere PRAIND");
        assert_eq!(datos.fecha_vencimiento_praind, None);
    }

    #[test]
    fn personal_de_ruta_siempre_requiere_praind() {
        let mut form = formulario_valido();
        form.tipo = TipoIngreso::Swat;
        form.es_personal_ruta = true;
        form.fecha_praind.clear();
        let errores = form.validar().err().expect("ruta requiere PRAIND");
        assert!(
            errores
                .iter()
                .any(|(campo, _)| *campo == Campo::FechaPraind)
        );
    }

    // ── Edición ──────────────────────────────────────────────────────────

    #[test]
    fn editar_precarga_todos_los_valores() {
        let form = FormularioContratista::editar(&resumen(), empresas(), true, true);
        assert_eq!(form.modo, ModoFormulario::Editar { id: 7 });
        assert_eq!(form.cedula, "119430546");
        assert_eq!(form.nombre, "Carlos Pérez");
        assert_eq!(form.empresa, Some((2, "Eléctrica Quesada".to_string())));
        assert_eq!(form.tipo, TipoIngreso::InHouse);
        assert_eq!(form.fecha_praind, "15/03/2027");
        assert!(!form.es_personal_ruta);
        assert!(form.tiene_acceso);
    }

    #[test]
    fn datos_actualizacion_espeja_la_validacion() {
        let mut form = FormularioContratista::editar(&resumen(), empresas(), true, true);
        let datos = form.datos_actualizacion().expect("precargado válido");
        assert_eq!(datos.cedula, "119430546");
        assert_eq!(datos.empresa_id, 2);
        assert_eq!(
            datos.fecha_vencimiento_praind,
            NaiveDate::from_ymd_opt(2027, 3, 15)
        );
    }
}
