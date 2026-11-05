# Participar en Reloaded Engine

Esta alpha es pública para recoger experiencias en más dispositivos. Son útiles tanto los errores como los reportes de sesiones que funcionan.

## Probar la alpha

Descarga el APK ARM64 firmado desde [Releases](https://github.com/FxxMorgan/Reloaded-Engine/releases). Importa tu copia del juego y prueba primero a 1×. Si pruebas 2×, anótalo por separado.

En [Issues](https://github.com/FxxMorgan/Reloaded-Engine/issues/new/choose) hay formularios para errores, experiencias en dispositivos, gamepads y propuestas. Incluye:

- Modelo del teléfono, Android y procesador/GPU si los conoces.
- Versión de Reloaded Engine y del juego.
- Escena, duración de la sesión y velocidad utilizada.
- Pasos para repetir un fallo y capturas o logs cuando sea posible.
- Modelo y tipo de conexión del gamepad, si lo usaste.

No adjuntes ejecutables, recursos del juego ni archivos de partidas. Las capturas y los logs suelen bastar para empezar el diagnóstico.

## Recoger logs por ADB

Con la depuración activada y el dispositivo conectado:

```bash
adb logcat -v threadtime -s ReloadedEGL NativeBridge AndroidRuntime > reloaded-engine.log
```

Reproduce el problema, detén la captura con Ctrl+C y adjunta el archivo al reporte. Si hay más de un dispositivo conectado, utiliza `adb -s DIRECCION_O_SERIE logcat ...`. Revisa el archivo antes de compartirlo para retirar cualquier dato personal.

El panel de diagnóstico se abre desde la flecha del juego y el botón DBG. Indica la escena y si el valor corresponde a FPS del motor o a pasos por segundo.

## Cambios de código

Lee [README.md](README.md) para compilar y [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) para conservar la atribución. Mantén cada PR centrada en un problema y explica cómo la comprobaste. No incluyas binarios de juego, claves de firma ni archivos de tu dispositivo.

La CI verifica el núcleo, el puente JNI, las pruebas de OpenGMK fuera de siete comparaciones numéricas conocidas, y los tests y lint de Android. La suite completa de OpenGMK se ejecuta además como diagnóstico visible; esos siete fallos siguen documentados y no cuentan como pruebas aprobadas.

Las releases se generan automáticamente al publicar un tag `alpha-*` que coincida con `versionName`. El APK de distribución usa una clave de firma persistente y se acompaña del código fuente y sumas SHA-256.
