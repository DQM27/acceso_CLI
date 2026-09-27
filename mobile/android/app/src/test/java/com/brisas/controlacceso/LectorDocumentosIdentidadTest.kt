package com.brisas.controlacceso

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LectorDocumentosIdentidadTest {

    @Test
    fun clasificaCedulaNacional() {
        val texto = "TRIBUNAL SUPREMO DE ELECCIONES\n1-1234-0567\nCOSTA RICA"
        assertEquals(TipoDocumento.CEDULA_NACIONAL, clasificarTipoDocumento(texto))
    }

    // --- Cédula nacional de frente (sin voltear al MRZ) ---
    // Basado en dos fotos reales del 2026-09-20: formato nuevo (con
    // orquídeas) y el formato azul anterior -- ambos comparten las mismas
    // etiquetas de campo, así que un solo extractor cubre los dos.

    @Test
    fun cedulaNacionalFrenteFormatoNuevoLeeNombreYApellidos() {
        val texto = """
            REPÚBLICA DE COSTA RICA
            TRIBUNAL SUPREMO DE ELECCIONES
            CÉDULA DE IDENTIDAD
            1 2345 6789
            Nombre: JUAN CARLOS
            1°Apellido: GOMEZ
            2°Apellido: VARGAS
            F. Nac: 22/08/2003 Vence: 08/04/2036
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CEDULA_NACIONAL, doc?.tipo)
        assertEquals("123456789", doc?.numeroDocumento)
        assertEquals("JUAN CARLOS", doc?.nombre)
        assertEquals("GOMEZ VARGAS", doc?.apellidos)
    }

    @Test
    fun cedulaNacionalFrenteFormatoAzulAnteriorLeeNombreYApellidos() {
        val texto = """
            REPÚBLICA DE COSTA RICA
            Tribunal Supremo de Elecciones
            Cédula de Identidad
            1 9876 5432
            Nombre: ANA LUCIA
            1° Apellido: RODRIGUEZ
            2° Apellido: SOLIS
            C.C.:
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CEDULA_NACIONAL, doc?.tipo)
        assertEquals("198765432", doc?.numeroDocumento)
        assertEquals("ANA LUCIA", doc?.nombre)
        assertEquals("RODRIGUEZ SOLIS", doc?.apellidos)
    }

    @Test
    fun cedulaNacionalFrenteSinNombreLegibleSigueDevolviendoElNumero() {
        // Ángulo/reflejo típico -- el número se leyó bien pero el bloque de
        // nombre/apellidos no calzó todavía. No debe bloquear la
        // confirmación por número (ver EstabilizadorLectura.mensajeDeConfirmacion,
        // que sí avisa "muéstreme el reverso" en este caso).
        val texto = "TRIBUNAL SUPREMO DE ELECCIONES\n1-1234-0567\nCOSTA RICA"

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CEDULA_NACIONAL, doc?.tipo)
        assertEquals("112340567", doc?.numeroDocumento)
        assertNull(doc?.nombre)
        assertNull(doc?.apellidos)
    }

    @Test
    fun clasificaCedulaResidencia() {
        val texto = "DIRECCIÓN GENERAL DE MIGRACIÓN Y EXTRANJERÍA\nRESIDENTE PERMANENTE"
        assertEquals(TipoDocumento.CEDULA_RESIDENCIA, clasificarTipoDocumento(texto))
    }

    @Test
    fun clasificaLicenciaNacional() {
        val texto = "Licencia de Conducir\nNº: 112340567"
        assertEquals(TipoDocumento.LICENCIA_NACIONAL, clasificarTipoDocumento(texto))
    }

    @Test
    fun clasificaLicenciaExtranjero() {
        val texto = "Licencia de Conducir\nNº: DM-999888777"
        assertEquals(TipoDocumento.LICENCIA_EXTRANJERO, clasificarTipoDocumento(texto))
    }

    @Test
    fun clasificaDesconocidoSinSenales() {
        val texto = "Documento sin ninguna palabra clave reconocible"
        assertEquals(TipoDocumento.DESCONOCIDO, clasificarTipoDocumento(texto))
    }

    @Test
    fun noClasificaCualquierNumeroDeNueveDigitosComoCedula() {
        val texto = "Cuenta de referencia 112340567"
        assertEquals(TipoDocumento.DESCONOCIDO, clasificarTipoDocumento(texto))
        assertNull(leerDocumentoDeTexto(texto))
    }

    @Test
    fun fechaInexistenteNoSeConstruyeComoDocumentoValido() {
        assertNull(FechaDocumento.crearValida(31, 2, 2030))
    }

    // --- Carnet de inducción PRAIND -- dos variantes de diseño reales ---

    @Test
    fun praindVarianteConEncabezadoCarnetDeInduccionAlSite() {
        val texto = """
            CARNET DE INDUCCIÓN AL SITE
            CEDIS COSTA RICA
            Nombre: Marco Anthony Jimenez Reyes
            No. de cédula: 155834532920
            Empresa: Expenic Ing S.A
            Fecha de inducción: 09/07/2026
            Fecha de vencimiento de
            inducción: 08/07/2028
            COCA COLA FEMSA COSTA RICA
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CARNET_INDUCCION_PRAIND, doc?.tipo)
        assertEquals("155834532920", doc?.numeroDocumento)
        assertEquals("Marco Anthony Jimenez Reyes", doc?.nombre)
        assertEquals("Expenic Ing S.A", doc?.empresa)
        assertEquals(FechaDocumento(8, 7, 2028), doc?.vencimiento)
    }

    @Test
    fun praindVarianteConPieDeNormasDeSeguridad() {
        // Segunda variante de diseño: el texto "CARNET DE INDUCCIÓN" está en
        // el pie de página, no en el encabezado, y el número de cédula acá
        // tiene 9 dígitos (el mismo largo que una cédula nacional) -- por
        // eso la clasificación de PRAIND tiene que ganarle a la de cédula
        // nacional, no sólo coincidir con ella.
        val texto = """
            Nombre: Mariela Cordero Salazar
            No. de cédula: 113850770
            Empresa: Expenic Ing S.A
            Fecha de inducción: 29/07/2026
            Fecha de vencimiento de
            inducción: 29/07/2027
            CARNET DE INDUCCIÓN EN NORMAS DE SEGURIDAD, AMBIENTE, CALIDAD E INOCUIDAD PARA CONTRATISTAS ML-SC-RGCR0369
            COCA COLA FEMSA COSTA RICA
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CARNET_INDUCCION_PRAIND, doc?.tipo)
        assertEquals("113850770", doc?.numeroDocumento)
        assertEquals("Mariela Cordero Salazar", doc?.nombre)
        assertEquals("Expenic Ing S.A", doc?.empresa)
        assertEquals(FechaDocumento(29, 7, 2027), doc?.vencimiento)
    }

    // --- Carnets de contratista in-house / BAC ---

    @Test
    fun inHouseFrenteBuscaPorNombreSiNoTraeCedulaVisible() {
        val texto = """
            Wardner Eduardo
            Marin Umaña
            CONTRATISTA
            COSTA RICA
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CARNET_IN_HOUSE, doc?.tipo)
        assertEquals("Wardner Eduardo Marin Umaña", doc?.textoBusqueda)
        assertEquals("Wardner Eduardo Marin Umaña", doc?.nombre)
    }

    @Test
    fun inHouseReversoBuscaPorCedula() {
        val texto = """
            EMPRESA:
            ALDAMA
            CEDULA:
            172400408127
            CONTRATISTA
            COSTA RICA
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CARNET_IN_HOUSE, doc?.tipo)
        assertEquals("172400408127", doc?.numeroDocumento)
    }

    @Test
    fun bacFrenteBuscaPorCedula() {
        val texto = """
            BAC
            ANTHONNY JOSE MURILLO RAMIREZ
            116030489
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CARNET_BAC, doc?.tipo)
        assertEquals("116030489", doc?.numeroDocumento)
        assertEquals("ANTHONNY JOSE MURILLO RAMIREZ", doc?.nombre)
    }

    @Test
    fun gafeteContratistaReconoceCodigoCrcSinUsarloComoCedula() {
        val texto = """
            CARNÉ
            PROVISIONAL
            CRC - 16
            CONTRATISTAS
            Costa Rica
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.GAFETE_CONTRATISTA, doc?.tipo)
        assertEquals("16", doc?.numeroDocumento)
        assertEquals("16", doc?.textoBusqueda)
    }

    @Test
    fun carneProvisionalPermisoLaboralBuscaPorDocumento() {
        val texto = """
            Carné Provisional - Permiso Laboral
            Expediente 135 788985
            N° Documento 155846198814
            CATEGORIA ESPECIAL R
            Primer Apellido CASTILLO
            Segundo Apellido MONTIEL
            Fecha Vencimiento 14/03/2027
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CEDULA_RESIDENCIA, doc?.tipo)
        assertEquals("155846198814", doc?.numeroDocumento)
        assertEquals(FechaDocumento(14, 3, 2027), doc?.vencimiento)
    }

    // --- Caso central del plan: Documento No. vs Expediente No. ---

    @Test
    fun dimexExtraeDocumentoNoExpediente() {
        val texto = """
            DIRECCIÓN GENERAL DE MIGRACIÓN Y EXTRANJERÍA
            REPÚBLICA DE COSTA RICA
            RESIDENTE PERMANENTE
            LIBRE CONDICIÓN
            Apellidos:
            PEREZ RAMIREZ
            Nombre:
            MARIA JOSE
            Nacionalidad:
            NICARAGUA
            Documento No.: 999888777
            Expediente No.: 135-453544
            Vence: 28 07 2026
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CEDULA_RESIDENCIA, doc?.tipo)
        assertEquals("999888777", doc?.numeroDocumento)
        assertEquals("MARIA JOSE", doc?.nombre)
        assertEquals("PEREZ RAMIREZ", doc?.apellidos)
        assertNull(doc?.nacionalidad) // el frente del DIMEX ya no la lee
        assertEquals(FechaDocumento(28, 7, 2026), doc?.vencimiento)
    }

    @Test
    fun dimexNoConfundeConNumeroDeExpedienteSiApareceAntes() {
        val texto = "Cédula de residencia\nExpediente No.: 135-453544\nDocumento No.: 999888777"
        val doc = leerDocumentoDeTexto(texto)
        assertEquals("999888777", doc?.numeroDocumento)
    }

    // --- Licencias ---

    @Test
    fun licenciaNacionalNoMarcaExtranjero() {
        val texto = "Licencia de Conducir\nNº: 112340567\nVencimiento 03-04-2026"
        val doc = leerDocumentoDeTexto(texto)
        assertEquals(TipoDocumento.LICENCIA_NACIONAL, doc?.tipo)
        assertEquals("112340567", doc?.numeroDocumento)
        assertEquals(false, doc?.esExtranjero)
    }

    @Test
    fun licenciaNacionalAceptaPrefijoCi() {
        val texto = """
            REPUBLICA DE COSTA RICA
            Licencia de Conducir
            N° CI-205300606
            Expedición 08-08-2024
            Nacimiento 04-02-1978
            Vencimiento 08-08-2027
            Tipo B3
            GUTIERREZ MARTINEZ JUAN JOSE
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.LICENCIA_NACIONAL, doc?.tipo)
        assertEquals("205300606", doc?.numeroDocumento)
        assertEquals(false, doc?.esExtranjero)
        assertEquals(FechaDocumento(8, 8, 2027), doc?.vencimiento)
        // MV-04/hallazgo 2026-09-26: antes esto no se intentaba leer para
        // nada. Orden legal costarricense: 1er apellido, 2do apellido,
        // nombre -- "GUTIERREZ MARTINEZ" son los apellidos, "JUAN JOSE" el
        // nombre.
        assertEquals("JUAN JOSE", doc?.nombre)
        assertEquals("GUTIERREZ MARTINEZ", doc?.apellidos)
    }

    @Test
    fun licenciaExtranjeroLeeNombreYApellidosSinEtiqueta() {
        // Texto reconstruido de una licencia real de extranjero
        // (2026-09-26, reporte en vivo: "reconoce el número de cédula pero
        // no carga el nombre") -- sin ninguna etiqueta "Nombre:", el
        // nombre completo es la última línea de puras mayúsculas antes del
        // resto de campos administrativos/código de barras. Nombre
        // compuesto de tres palabras ("DANIEL DE JESUS") para confirmar
        // que no se trunca a una sola.
        val texto = """
            REPUBLICA DE COSTA RICA
            Licencia de Conducir
            Nº: DM-155824395105
            Expedición 03-04-2023
            Nacimiento 30-05-1989
            Tipo: A3
            Vencimiento 03-04-2026
            Donador
            R.F. R.T. T.S. NI.
            QUINTANA MEDINA DANIEL DE JESUS
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.LICENCIA_EXTRANJERO, doc?.tipo)
        assertEquals("155824395105", doc?.numeroDocumento)
        assertEquals("DANIEL DE JESUS", doc?.nombre)
        assertEquals("QUINTANA MEDINA", doc?.apellidos)
    }

    @Test
    fun licenciaExtranjeroRemuevePrefijoDM() {
        val texto = "Licencia de Conducir\nNº: DM-999888777\nVencimiento 03-04-2026"
        val doc = leerDocumentoDeTexto(texto)
        assertEquals(TipoDocumento.LICENCIA_EXTRANJERO, doc?.tipo)
        assertEquals("999888777", doc?.numeroDocumento)
        assertTrue(doc?.esExtranjero == true)
        assertEquals(FechaDocumento(3, 4, 2026), doc?.vencimiento)
    }

    @Test
    fun licenciaExtranjeroAceptaDmAunqueOcrPierdaEtiquetaNumero() {
        val texto = """
            REPUBLICA DE COSTA RICA
            Licencia de Conducir
            DM-155824395105
            Expedición 03-04-2023
            Vencimiento 03-04-2026
            Tipo A3
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.LICENCIA_EXTRANJERO, doc?.tipo)
        assertEquals("155824395105", doc?.numeroDocumento)
        assertTrue(doc?.esExtranjero == true)
        assertEquals(FechaDocumento(3, 4, 2026), doc?.vencimiento)
    }

    // --- Vigencia ---

    @Test
    fun documentoVigenteSiVencimientoEsFuturo() {
        val vencimiento = FechaDocumento(28, 7, 2026)
        val hoy = FechaDocumento(8, 9, 2025)
        assertEquals(false, vencimiento.estaVencida(hoy))
    }

    @Test
    fun documentoVencidoSiVencimientoEsPasado() {
        val vencimiento = FechaDocumento(28, 7, 2024)
        val hoy = FechaDocumento(8, 9, 2026)
        assertEquals(true, vencimiento.estaVencida(hoy))
    }

    // --- Casos sin match ---

    @Test
    fun devuelveNullSiNoHaySuficienteInformacion() {
        assertNull(leerDocumentoDeTexto("Documento sin numeros completos"))
    }

    // --- Edad / TIM vs cédula de adulto ---

    @Test
    fun edadCuandoYaPasoElCumpleañosEsteAnio() {
        val nacimiento = FechaDocumento(15, 6, 2000)
        val hoy = FechaDocumento(8, 9, 2026) // cumpleaños (15/06) ya pasó
        assertEquals(26, nacimiento.edadEnAnios(hoy))
    }

    @Test
    fun edadCuandoAunNoLlegaElCumpleañosEsteAnio() {
        val nacimiento = FechaDocumento(15, 12, 2000)
        val hoy = FechaDocumento(8, 9, 2026) // cumpleaños (15/12) todavía no llega
        assertEquals(25, nacimiento.edadEnAnios(hoy))
    }

    @Test
    fun edadElMismoDiaDelCumpleañosYaCuentaComoCumplida() {
        val nacimiento = FechaDocumento(8, 9, 2000)
        val hoy = FechaDocumento(8, 9, 2026)
        assertEquals(26, nacimiento.edadEnAnios(hoy))
    }

    @Test
    fun reclasificaCedulaNacionalDeMenorComoTim() {
        val documento = DocumentoDetectado(
            tipo = TipoDocumento.CEDULA_NACIONAL,
            numeroDocumento = "202020202",
            fechaNacimiento = FechaDocumento(15, 6, 2018),
        )
        val hoy = FechaDocumento(8, 9, 2026) // 8 años
        assertEquals(TipoDocumento.TARJETA_IDENTIDAD_MENOR, documento.reclasificarPorEdad(hoy).tipo)
    }

    @Test
    fun noReclasificaCedulaNacionalDeAdulto() {
        val documento = DocumentoDetectado(
            tipo = TipoDocumento.CEDULA_NACIONAL,
            numeroDocumento = "101110111",
            fechaNacimiento = FechaDocumento(15, 6, 1990),
        )
        val hoy = FechaDocumento(8, 9, 2026)
        assertEquals(TipoDocumento.CEDULA_NACIONAL, documento.reclasificarPorEdad(hoy).tipo)
    }

    @Test
    fun noReclasificaOtrosTiposAunqueSeanMenoresDeEdad() {
        // La regla es específica de cédula nacional -- DIMEX/licencia/pasaporte
        // de un menor no deben convertirse en TIM (no es lo que son).
        val documento = DocumentoDetectado(
            tipo = TipoDocumento.CEDULA_RESIDENCIA,
            numeroDocumento = "999888777",
            fechaNacimiento = FechaDocumento(15, 6, 2018),
        )
        val hoy = FechaDocumento(8, 9, 2026)
        assertEquals(TipoDocumento.CEDULA_RESIDENCIA, documento.reclasificarPorEdad(hoy).tipo)
    }

    @Test
    fun noReclasificaSiNoHayFechaDeNacimiento() {
        val documento = DocumentoDetectado(tipo = TipoDocumento.CEDULA_NACIONAL, numeroDocumento = "101110111")
        val hoy = FechaDocumento(8, 9, 2026)
        assertEquals(TipoDocumento.CEDULA_NACIONAL, documento.reclasificarPorEdad(hoy).tipo)
    }

    // --- DIMEX: nombre/apellidos no deben quedar en un valor de Sexo ---

    @Test
    fun dimexNoConfundeNombreConValorDeSexo() {
        // Reproduce el hallazgo real (2026-09-20): ML Kit linealiza el
        // texto de un DIMEX real con "Nombre:" seguido del valor de Sexo,
        // no del nombre real -- antes esto quedaba guardado tal cual
        // ("MASCULINO") en el campo `nombre`.
        val texto = """
            RESIDENTE PERMANENTE
            DOCUMENTO NO.: 155824395105
            Nombre:
            MASCULINO
            Apellidos:
            FEMENINO
            Nacionalidad:
            NICARAGUENSE
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CEDULA_RESIDENCIA, doc?.tipo)
        assertNull(doc?.nombre)
        assertNull(doc?.apellidos)
        // El frente del DIMEX ya no lee nacionalidad (sólo nombre,
        // apellidos, número y vencimiento).
        assertNull(doc?.nacionalidad)
    }

    @Test
    fun dimexMantieneNombreYApellidosCuandoNoHayColision() {
        val texto = """
            RESIDENTE PERMANENTE
            DOCUMENTO NO.: 155824395105
            Nombre:
            JUAN CARLOS
            Apellidos:
            PEREZ MORA
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals("JUAN CARLOS", doc?.nombre)
        assertEquals("PEREZ MORA", doc?.apellidos)
    }

    // --- DIMEX: "Género: M" en la misma línea que el nombre ---

    @Test
    fun dimexIgnoraGeneroEnLaMismaLineaDelNombre() {
        // Layout real de la DIMEX: "Género: M" está a la derecha del nombre,
        // y ML Kit los devuelve en la misma línea.
        val texto = """
            RESIDENTE PERMANENTE
            LIBRE CONDICIÓN
            Apellidos:
            PEREZ MORA
            Nombre:
            JUAN CARLOS Género: M
            Nacionalidad:
            NICARAGUA
            Documento No.: 155824395105
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals("JUAN CARLOS", doc?.nombre)
        assertEquals("PEREZ MORA", doc?.apellidos)
    }

    @Test
    fun dimexIgnoraGeneroSinTildeOLeidoRaro() {
        for (linea in listOf("JUAN CARLOS Genero: F", "JUAN CARLOS GÉNERO M", "JUAN CARLOS Gnero: M", "Nombre: JUAN CARLOS   Sexo: M")) {
            val texto = "RESIDENTE PERMANENTE\nDocumento No.: 155824395105\n" +
                (if (linea.startsWith("Nombre:")) linea else "Nombre:\n$linea")
            assertEquals(linea, "JUAN CARLOS", leerDocumentoDeTexto(texto)?.nombre)
        }
    }

    @Test
    fun dimexNoCortaNombresQueEmpiezanComoGenero() {
        val texto = "RESIDENTE PERMANENTE\nDocumento No.: 155824395105\nNombre:\nGENEROSO ANTONIO"
        assertEquals("GENEROSO ANTONIO", leerDocumentoDeTexto(texto)?.nombre)
    }

    // --- DIMEX: esquema completo, cada renglón visual con su campo vecino ---

    @Test
    fun dimexLeeTarjetaCompletaConCamposVecinosEnElMismoRenglon() {
        // Así devuelve ML Kit el frente real: un renglón por línea visual,
        // con la columna derecha pegada al valor de la izquierda.
        val texto = """
            DIRECCIÓN GENERAL DE MIGRACIÓN Y EXTRANJERÍA
            REPÚBLICA DE COSTA RICA
            RESIDENTE PERMANENTE
            LIBRE CONDICIÓN
            Apellidos:
            PEREZ MORA
            Nombre:
            JUAN CARLOS Género: M
            Nacionalidad:
            NICARAGUA F.nac.: 01 01 1990
            Documento No.: 155800000001 Emitido: 01 01 2023
            Expediente No.: 135 - 000000 Vence: 01 01 2027
            DGME
            DOCUMENTO DE IDENTIDAD MIGRATORIO PARA EXTRANJEROS
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CEDULA_RESIDENCIA, doc?.tipo)
        assertEquals("155800000001", doc?.numeroDocumento)
        assertEquals("JUAN CARLOS", doc?.nombre)
        assertEquals("PEREZ MORA", doc?.apellidos)
        assertNull(doc?.nacionalidad) // se descarta a propósito
        assertEquals(1, doc?.vencimiento?.dia)
        assertEquals(2027, doc?.vencimiento?.anio)
    }

    @Test
    fun dimexConEtiquetaYGeneroEnUnRenglonYNombreEnElSiguiente() {
        val texto = """
            RESIDENTE PERMANENTE
            Apellidos: PEREZ MORA
            Nombre: Género: M
            JUAN CARLOS
            Nacionalidad: NICARAGUA
            Documento No.: 155800000001
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals("JUAN CARLOS", doc?.nombre)
        assertEquals("PEREZ MORA", doc?.apellidos)
    }

    @Test
    fun dimexSinValorNoRobaLaEtiquetaSiguiente() {
        val texto = "RESIDENTE PERMANENTE\nApellidos:\nNombre:\nJUAN CARLOS\nDocumento No.: 155800000001"
        val doc = leerDocumentoDeTexto(texto)
        assertEquals(null, doc?.apellidos)
        assertEquals("JUAN CARLOS", doc?.nombre)
    }

    @Test
    fun dimexNumeroSeparadoDeSuEtiquetaUsaElDeOnceODoceDigitosNuncaElExpediente() {
        val texto = """
            RESIDENTE PERMANENTE
            Documento No.: Emitido: 01 01 2023
            Expediente No.: 135 - 000000
            155800000001
        """.trimIndent()
        assertEquals("155800000001", leerDocumentoDeTexto(texto)?.numeroDocumento)
    }

    // --- Licencia: esquema completo, sólo número/nombre/apellidos/vencimiento ---

    @Test
    fun licenciaExtranjeroIgnoraPieDeImpresionYNumerosSueltos() {
        // Frente real completo, con el pie "... N0940950 ... BCR GOB DIGITAL"
        // partido en líneas como a veces lo devuelve ML Kit.
        val texto = """
            REPUBLICA DE COSTA RICA
            Licencia de Conducir
            Nº: DM-155800000001
            Expedición 03-04-2023
            Nacimiento 30-05-1989
            Tipo: A3
            Vencimiento 03-04-2026
            Donador
            R.F. R.T. T.S. NI.
            PEREZ MORA JUAN CARLOS
            DIRECCION GENERAL EDUCACION VIAL MOPT
            08770445
            053323202302
            03/04/2023 11:54 PR-C151 N0940950 830
            BCR GOB DIGITAL
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.LICENCIA_EXTRANJERO, doc?.tipo)
        assertEquals("155800000001", doc?.numeroDocumento)
        assertEquals("JUAN CARLOS", doc?.nombre)
        assertEquals("PEREZ MORA", doc?.apellidos)
        assertEquals(FechaDocumento(3, 4, 2026), doc?.vencimiento)
        assertNull(doc?.fechaNacimiento)
    }

    @Test
    fun licenciaNoTomaElNumeroDelPieSiFaltaElSimboloDeNumero() {
        val texto = """
            Licencia de Conducir
            CI-205300606
            Vencimiento 08-08-2027
            03/04/2023 11:54 PR-C151 N0940950 830
        """.trimIndent()
        assertEquals("205300606", leerDocumentoDeTexto(texto)?.numeroDocumento)
    }

    @Test
    fun licenciaNacionalConCedulaConGuiones() {
        val texto = "Licencia de Conducir\nNº: 1-1234-0567\nVencimiento 03-04-2026"
        assertEquals("112340567", leerDocumentoDeTexto(texto)?.numeroDocumento)
    }

    @Test
    fun licenciaRespetaApellidosCompuestosConParticulas() {
        val texto = "Licencia de Conducir\nNº: CI-205300606\nVencimiento 08-08-2027\nDE LA O CASTRO ANA MARIA"
        val doc = leerDocumentoDeTexto(texto)
        assertEquals("ANA MARIA", doc?.nombre)
        assertEquals("DE LA O CASTRO", doc?.apellidos)
    }

    // --- Cédula nacional: esquema, bloques separados y sólo 4 campos ---

    @Test
    fun cedulaNuevaLeeNombreApellidosNumeroYVenceIgnorandoFechaDeNacimiento() {
        val texto = """
            REPÚBLICA DE COSTA RICA
            TRIBUNAL SUPREMO DE ELECCIONES
            CÉDULA DE IDENTIDAD
            1 2345 6789
            JUAN.C.G
            Nombre: JUAN CARLOS
            1ºApellido: GOMEZ
            2ºApellido: VARGAS F. Nac:22/08/2003 Vence:08/04/2036
            22/08/2003
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals("123456789", doc?.numeroDocumento)
        assertEquals("JUAN CARLOS", doc?.nombre)
        assertEquals("GOMEZ VARGAS", doc?.apellidos)
        assertEquals(FechaDocumento(8, 4, 2036), doc?.vencimiento)
        assertNull(doc?.fechaNacimiento)
    }

    @Test
    fun cedulaAzulConEtiquetasYValoresEnBloquesSeparados() {
        // Etiquetas alineadas a la derecha en su propia columna: ML Kit
        // devuelve primero todas las etiquetas y después todos los valores.
        val texto = """
            REPÚBLICA DE COSTA RICA
            Tribunal Supremo de Elecciones
            Cédula de Identidad
            1 9876 5432
            Ana L.S
            Nombre:
            1° Apellido:
            2° Apellido:
            C.C:
            ANA LUCIA
            RODRIGUEZ
            SOLIS
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals("198765432", doc?.numeroDocumento)
        assertEquals("ANA LUCIA", doc?.nombre)
        assertEquals("RODRIGUEZ SOLIS", doc?.apellidos)
        assertNull(doc?.vencimiento)
    }

    @Test
    fun cedulaAzulConValoresAntesQueLasEtiquetas() {
        val texto = """
            REPÚBLICA DE COSTA RICA
            Cédula de Identidad
            1 9876 5432
            ANA LUCIA
            RODRIGUEZ
            SOLIS
            Nombre:
            1° Apellido:
            2° Apellido:
            C.C:
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals("ANA LUCIA", doc?.nombre)
        assertEquals("RODRIGUEZ SOLIS", doc?.apellidos)
    }

    @Test
    fun cedulaNoTomaLaEtiquetaSiguienteComoValor() {
        val texto = "Cédula de Identidad\n1 9876 5432\nNombre:\n1° Apellido: RODRIGUEZ\n2° Apellido: SOLIS"
        val doc = leerDocumentoDeTexto(texto)
        assertNull(doc?.nombre)
        assertEquals("RODRIGUEZ SOLIS", doc?.apellidos)
    }

    // --- In House en estuche: "COSTA RICA" perdido y basura del reflejo ---

    @Test
    fun inHouseSinCostaRicaSeReconocePorLaFranjaContratista() {
        val texto = """
            Ana Maria
            Muñoz Rojas
            CONTRATISTA
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CARNET_IN_HOUSE, doc?.tipo)
        assertEquals("Ana Maria Muñoz Rojas", doc?.nombre)
    }

    @Test
    fun inHouseIgnoraBasuraDelReflejoDelEstuche() {
        val texto = """
            Ana Maria
            Muñoz Rojas
            l.
            CONTRATISTA
            COSTA RICA
        """.trimIndent()

        assertEquals("Ana Maria Muñoz Rojas", leerDocumentoDeTexto(texto)?.nombre)
    }

    // --- Reverso de la cédula azul anterior ---

    private val reversoCedulaAnterior = """
        Número de Cédula: 1 2345 6789
        Fecha de Nacimiento: 01 01 1970
        Lugar de Nacimiento: TURRUBARES SAN JOSE
        Nombre del Padre: JUAN PEREZ MORA
        Nombre de la Madre: ANA ROJAS VEGA
        Domicilio Electoral: RINCON CENTRAL ALAJUELA
        Vencimiento: 18 10 2028
        Sexo:
        001234567
    """.trimIndent()

    @Test
    fun reversoCedulaAnteriorLeeNumeroYVencimientoSinNombresDeLosPadres() {
        val doc = leerDocumentoDeTexto(reversoCedulaAnterior)

        assertEquals(TipoDocumento.CEDULA_NACIONAL, doc?.tipo)
        assertEquals("123456789", doc?.numeroDocumento)
        assertEquals(FechaDocumento(18, 10, 2028), doc?.vencimiento)
        assertNull(doc?.nombre)
        assertNull(doc?.apellidos)
    }

    @Test
    fun reversoCedulaAnteriorConEtiquetasYValoresEnBloquesSeparados() {
        val texto = """
            Número de Cédula:
            Fecha de Nacimiento:
            Lugar de Nacimiento:
            Nombre del Padre:
            Nombre de la Madre:
            Domicilio Electoral:
            Vencimiento: 18 10 2028
            1 2345 6789
            01 01 1970
            TURRUBARES SAN JOSE
            JUAN PEREZ MORA
            ANA ROJAS VEGA
            RINCON CENTRAL ALAJUELA
            001234567
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals("123456789", doc?.numeroDocumento)
        assertNull(doc?.nombre)
        assertNull(doc?.apellidos)
    }

    @Test
    fun reversoCedulaAnteriorNoTomaElNumeroDeControlQueEmpiezaEnCero() {
        val texto = reversoCedulaAnterior.replace("Número de Cédula: 1 2345 6789\n", "")
        assertNull(leerDocumentoDeTexto(texto))
    }

    // --- Cédula nueva real (2026-09-27): etiquetas pegadas al valor ---

    @Test
    fun cedulaNuevaConEtiquetasPegadasAlValor() {
        val texto = """
            REPÚBLICA DE COSTA RICA
            TRIBUNAL SUPREMO DE ELECCIONES
            CÉDULA DE IDENTIDAD
            1 0000 0219
            Nombre:JUAN
            1ºApellido:PEREZ
            2ºApellido:MORA
            F. Nac:26/03/1969 Vence:27/05/2036
            26/03/1969
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals("100000219", doc?.numeroDocumento)
        assertEquals("JUAN", doc?.nombre)
        assertEquals("PEREZ MORA", doc?.apellidos)
        assertEquals(FechaDocumento(27, 5, 2036), doc?.vencimiento)
    }

    @Test
    fun reversoCedulaNuevaNoSeLeeComoFrente() {
        // El reverso nuevo trae "Nombre: <nombre completo>", número y MRZ;
        // se resuelve por el MRZ (con checksum) en el estabilizador, no
        // por el extractor del frente.
        val texto = """
            Nombre: JUAN PEREZ MORA
            C.C.:
            1 0000 0219
            TSECR
            C004780077
            IDCRI1000002190<C004780077<<<<
        """.trimIndent()
        assertNull(leerDocumentoDeTexto(texto))
    }

    // --- Gafete de contratista (CRC) ---

    @Test
    fun gafeteContratistaConCarneProvisionalEnUnRenglonNoSeConfundeConDimex() {
        val texto = "CARNÉ PROVISIONAL\nCRC - 12\nCONTRATISTAS\nCosta Rica"
        val doc = leerDocumentoDeTexto(texto)
        assertEquals(TipoDocumento.GAFETE_CONTRATISTA, doc?.tipo)
        assertEquals("12", doc?.numeroDocumento)
    }

    @Test
    fun gafeteContratistaSinCostaRicaNiFranjaVerdeSeReconocePorCrcYProvisional() {
        val doc = leerDocumentoDeTexto("CARNÉ\nPROVISIONAL\nCRC - 12")
        assertEquals(TipoDocumento.GAFETE_CONTRATISTA, doc?.tipo)
        assertEquals("12", doc?.numeroDocumento)
    }

    // --- PRAIND vertical: título/pie fuera del recuadro horizontal ---

    @Test
    fun praindSinTituloNiPieSeReconocePorSusEtiquetas() {
        val texto = """
            Nombre: Ana Maria Rojas Vega
            No. de cédula: 701000000
            Empresa: Sodexo
            Fecha de inducción: 03/08/2026
            Fecha de vencimiento de
            inducción: 03/08/2027
        """.trimIndent()

        val doc = leerDocumentoDeTexto(texto)

        assertEquals(TipoDocumento.CARNET_INDUCCION_PRAIND, doc?.tipo)
        assertEquals("701000000", doc?.numeroDocumento)
        assertEquals("Ana Maria Rojas Vega", doc?.nombre)
        assertEquals("Sodexo", doc?.empresa)
        assertEquals(FechaDocumento(3, 8, 2027), doc?.vencimiento)
    }
}
