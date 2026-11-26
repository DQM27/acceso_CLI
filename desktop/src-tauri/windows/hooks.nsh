; Pasos propios del instalador de Windows (bundle > windows > nsis >
; installerHooks en tauri.conf.json). Solo ASCII: makensis rechaza las
; tildes de un .nsh sin BOM ("Bad text encoding").
;
; Al terminar de instalar o actualizar, avisa a Windows que los iconos
; cambiaron. Si no, el acceso directo del escritorio y el de la barra de
; tareas siguen mostrando el icono viejo desde la cache de iconos de
; Windows, aunque el programa ya traiga el nuevo (pasaba con el icono
; de Navidad, 2026-11-26). SHChangeNotify(SHCNE_ASSOCCHANGED) es el aviso
; estandar; ie4uinit -show renueva la cache del usuario en Windows 10/11.
!macro NSIS_HOOK_POSTINSTALL
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
  nsExec::Exec '"$SYSDIR\ie4uinit.exe" -show'
  Pop $0
!macroend
