package com.brisas.controlacceso

import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.text.input.KeyboardType
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class InterfazPrincipalTest {
    @Test
    fun `el subtitulo dice la unidad y cuantos hay adentro`() {
        assertEquals("PLANTA NORTE · 12 adentro", subtituloSeccion("PLANTA NORTE", SeccionPrincipal.ACTIVOS, 12))
        assertEquals("0 adentro", subtituloSeccion(null, SeccionPrincipal.EXTERNOS, 0))
    }

    @Test
    fun `en KOF cuenta gafetes prestados, en singular y plural`() {
        assertEquals("PLANTA NORTE · 1 gafete prestado", subtituloSeccion("PLANTA NORTE", SeccionPrincipal.KOF, 1))
        assertEquals("3 gafetes prestados", subtituloSeccion(null, SeccionPrincipal.KOF, 3))
    }

    @Test
    fun `la ficha de salida lista los datos en orden y destaca el gafete`() {
        val datos = datosIngreso(
            cedula = "111111111",
            afiliacion = "Empresa" to "Empresa Test",
            placa = "CL 371931",
            gafete = 25,
            fechaHora = "2026-10-25 08:00",
            dioIngreso = "Guardia",
            otroEquipo = false,
            hora = { "hoy 08:00" },
        )
        assertEquals(
            listOf("Cédula", "Empresa", "Placa", "Gafete", "Ingresó", "Dio ingreso"),
            datos.map { it.etiqueta },
        )
        assertEquals(DatoFicha("Gafete", "25", destacado = true), datos[3])
        assertEquals("hoy 08:00", datos[4].valor)
    }

    @Test
    fun `la ficha omite lo que falta y marca el otro equipo`() {
        val datos = datosIngreso(
            cedula = null,
            afiliacion = "Motivo" to "",
            gafete = null,
            fechaHora = "x",
            dioIngreso = null,
            otroEquipo = true,
            hora = { it },
        )
        assertEquals(listOf("Gafete", "Ingresó", "Equipo"), datos.map { it.etiqueta })
        assertEquals("Sin gafete", datos[0].valor)
        assertNull(datos.firstOrNull { it.etiqueta == "Cédula" })
    }

    @Test
    fun `el buscador de Activos cambia de ayuda y teclado con el modo`() {
        assertEquals("Nombre o cédula para ingresar", ayudaBusquedaActivos(ModoBusqueda.ENTRADA))
        assertEquals("Nombre o cédula de quien sale", ayudaBusquedaActivos(ModoBusqueda.SALIDA_NOMBRE))
        assertEquals("Gafetes: 2, 25, 85", ayudaBusquedaActivos(ModoBusqueda.SALIDA_GAFETE))
        assertEquals(KeyboardType.Number, tecladoBusquedaActivos(ModoBusqueda.SALIDA_GAFETE).keyboardType)
        assertEquals(KeyboardOptions.Default, tecladoBusquedaActivos(ModoBusqueda.ENTRADA))
    }

    @Test
    fun `cada toque de la interfaz usa un tipo distinto del sistema`() {
        val tipos = Haptico.entries.map(::tipoHaptico)
        assertEquals(tipos.size, tipos.toSet().size)
    }
}
