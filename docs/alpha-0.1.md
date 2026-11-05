# Reloaded Engine · alpha-0.1

Primera alpha pública del launcher para Android ARM64. Pokémon Reloaded 1.9.2.2 es el primer juego probado. La biblioteca mantiene una sola instalación; los perfiles para varios juegos están previstos para una etapa posterior.

## Descargar y participar

Instala `Reloaded-Engine-alpha-0.1-arm64.apk` desde los archivos de esta release. Está firmado para distribución y no incluye el juego. El código fuente correspondiente, el certificado público y las sumas SHA-256 se adjuntan por separado.

Queremos recoger experiencias en más dispositivos. Usa [las plantillas de Issues](https://github.com/FxxMorgan/Reloaded-Engine/issues/new/choose) para errores, resultados de dispositivos, gamepads o propuestas. Incluye modelo, Android, versión del juego y velocidad 1×/2×.

## Cambios destacados

- Portada centrada en Reloaded Engine, con jerarquía más clara y temas persistentes.
- Partidas reales y exportación ZIP de sus componentes, sin datos de ejemplo.
- Panel de diagnóstico oculto y barra de herramientas recogida al iniciar el juego.
- Eliminado el modo 4×; disponibles 1× y 2×.
- Soporte de gamepad Bluetooth/USB con cruceta, stick y botones físicos; los controles táctiles se ocultan al detectar el mando. Falta ampliar la validación con modelos reales.
- Correcciones de persistencia de salas, carga de sprites externos, transparencias, colisiones y temporización de diálogos.
- Los errores de extracción se propagan a la interfaz. La actualización del paquete conserva los archivos de partida existentes.
- Compilación nativa sin rutas personales y con fallo explícito si no se genera la biblioteca.

## Evidencia disponible

| Comprobación | Resultado |
| --- | --- |
| Carga de partida y cambios de sala | Verificados durante las pruebas en dispositivo |
| Combate tras corregir sprites | Completado y confirmado por el probador |
| Rendimiento de ese combate | Aproximadamente 29 pasos/s en el intervalo registrado |
| Decodificación de recursos | 26.237 sprites, 103.602 cuadros, cero errores |
| Regresiones del cargador y transparencia | 11 pruebas nativas de sprites aprobadas |
| Núcleo de importación y utilidades | 27 pruebas aprobadas |
| Archivo ZIP de partidas | 2 pruebas JVM aprobadas; exportación real verificada, 7 archivos idénticos byte a byte |
| Puente JNI y ciclo de vida auxiliar | 13 pruebas aprobadas |
| APK debug, release y lint | Compilación con JDK 21; lint sin errores |
| Copia local en dispositivo | Creada y comparada con la exportación externa; contenido idéntico |
| Suite completa de OpenGMK en Linux | 7 fallos de precisión numérica pendientes |

Los logs de desarrollo se conservan localmente en `build/diagnostics/`. Los guardados, recursos del juego y archivos de diagnóstico privados no deben incorporarse a una publicación del código.

## Distribución y límites de la prueba

La publicación incluye un APK firmado, código fuente con los cambios al motor, avisos de terceros y sumas SHA-256. La firma se conserva para futuras actualizaciones. Las diferencias con la referencia de Klisee se explican en [la comparación del proyecto](https://github.com/FxxMorgan/Reloaded-Engine/blob/main/docs/comparacion-klisee.md).

Queremos ampliar estas comprobaciones con los reportes de la alpha pública:

- Completar la prueba de instalación desde cero con el ZIP completo.
- Validar una sesión prolongada, retorno desde segundo plano y al menos otro dispositivo ARM64.
- Investigar las siete pruebas numéricas fallidas de OpenGMK en Linux antes de presentar la suite completa como aprobada.

La comprobación inicial se realizó en un Motorola edge 40. Aún no acredita estabilidad general ni una experiencia completa del juego. La carga del SoundFont empaquetado y la verificación de música MIDI siguen pendientes en la app.
