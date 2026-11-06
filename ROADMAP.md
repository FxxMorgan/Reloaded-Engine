# Roadmap de Reloaded Engine

Actualizado el **5 de noviembre de 2026**.

Reloaded Engine busca convertirse en un launcher de juegos GameMaker 8 para Android. La alpha pública actual mantiene una instalación y tiene Pokémon Reloaded como primer juego probado. El trabajo inmediato se centra en conservar el progreso, completar la entrada y estabilizar las sesiones antes de ampliar la biblioteca.

Las fases describen prioridades y criterios de salida. No son fechas comprometidas. Un bloqueo de progreso o pérdida de datos confirmado puede adelantar cualquier tarea.

## Estado actual

- **Disponible en alpha-0.1:** importación ZIP, controles táctiles y gamepad, temas claro/oscuro, ejecución a 1×/2× y copias de partidas ZIP.
- **Preparado en desarrollo:** corrección de teclas retenidas al limpiar la entrada, con pruebas del estado real de teclado de GameMaker y limpieza en Android. Pendiente de comprobar con mando y dispositivo físicos y de incluirse en un APK posterior.
- **Pendiente confirmado en código:** diálogos de texto/confirmación del kernel, conexión del SoundFont al motor, varias teclas/acciones auxiliares y persistencia completa de la configuración de controles.
- **Por reproducir:** cierres de sesiones largas, degradación al abrir menús, fallos específicos de Pokédex, Galería de la Fama y algunas escenas del juego.

La [revisión de las 63 issues abiertas de Klisee](docs/triage-klisee.md) documenta las fuentes, las coincidencias y lo que ya existe en nuestra implementación. Sus reportes se utilizan como casos de prueba; una coincidencia de síntomas no basta para dar un fallo por confirmado o corregido aquí.

## Fase 1 · Próximos parches: progreso y sesiones seguras

| Prioridad | Trabajo | Criterio de comprobación |
| --- | --- | --- |
| P0 | Liberar entradas al perder foco, pausar, desconectar el mando, destruir la superficie o cancelar el joystick. | Las teclas activas se liberan en GameMaker, los taps cancelados no reaparecen y los mismos controles funcionan al volver. Pruebas automatizadas preparadas; falta comprobación física. |
| P0 | Restaurar copias ZIP y hacer transaccional la importación. | Una importación corrupta o interrumpida conserva una instalación utilizable y sus partidas; una copia restaurada conserva todos sus componentes y bytes. |
| P0 | Implementar confirmación y entrada de texto de Android para las funciones pendientes del kernel. | Cancelar, aceptar vacío y escribir texto no bloquean el motor; confirmar una acción siempre corresponde a la elección del usuario. |
| P0 | Reproducir cierres y degradación de rendimiento con logs de nuestra aplicación. | Casos repetibles de cambio de sala, bolsa/equipo y segundo plano; registrar tiempos de cuadro, memoria e instancias antes de elegir un parche. |
| P1 | Verificar coherencia de fechas y zonas horarias entre PC y Android. | Pruebas con copias de partida y reloj correcto, sin modificar el progreso original para ocultar un error. |

Para publicar el siguiente parche: CI requerida aprobada, firma de distribución conservada y prueba en dispositivo de cada flujo modificado. El APK público `alpha-0.1` permanece como referencia; los cambios en `main` no se anuncian como incluidos en él.

## Fase 2 · Controles completos y accesibilidad del juego

- Añadir una vía de entrada para teclas auxiliares como B, Q y AltGr, botones de ratón y desplazamiento, con accesos que no saturen la pantalla.
- Ofrecer mapeo de gamepad por dispositivo y juego, con ejes, cruceta, gatillos analógicos, reconexión y navegación de menús.
- Guardar distribución, tamaño, opacidad y modo de joystick entre reinicios.
- Revisar Safari, PC/cajas, Pokédex, selección de personajes y mapa con los controles configurados.
- Mantener comprobadas las diagonales y el joystick dinámico que ya existen.

**Criterio de salida:** completar esas escenas con pantalla táctil y al menos dos modelos de gamepad; documentar las asignaciones y conservar los ajustes al reiniciar.

## Fase 3 · Audio, gráficos y sesiones largas

- Cargar el SoundFont empaquetado y verificar reproducción MIDI real.
- Recuperar la salida de audio ante cambio de auriculares/Bluetooth/USB y errores de AudioTrack; definir el comportamiento de foco de audio.
- Perfilar el caso de Túnel Gaara y las aperturas repetidas de bolsa/equipo, además de sesiones prolongadas y retorno desde segundo plano.
- Verificar superficies guardadas, Galería de la Fama y sprites de huevos en las vistas correspondientes.
- Investigar las siete comparaciones numéricas heredadas de OpenGMK con la semántica de GameMaker como referencia; mantener visible la auditoría mientras sigan pendientes.

**Criterio de salida:** resultados reproducibles en más de una familia de GPU, una sesión prolongada con métricas conservadas y pruebas de las rutas de audio y gráficos modificadas. No se promete una tasa de FPS universal.

## Fase 4 · Biblioteca de varios juegos

- Separar instalación, partidas, configuración y caché por juego.
- Definir perfiles de compatibilidad para cada título y controles independientes.
- Añadir, actualizar y retirar un juego sin afectar a los demás.
- Incorporar un segundo juego GameMaker 8 con pruebas propias y una ficha pública de compatibilidad.

**Criterio de salida:** mantener dos instalaciones con progreso y ajustes aislados y comprobar que una actualización de una de ellas conserva la otra.

## Después de estabilizar la base

Traducciones de la interfaz, filtros gráficos y herramientas de diagnóstico compartibles. Multijugador y mods requieren investigación separada y se evaluarán después de los bloqueos de progreso, controles y estabilidad; no forman parte del próximo parche.

## Cómo ayudar a priorizar

Usa [las plantillas de Issues](https://github.com/FxxMorgan/Reloaded-Engine/issues/new/choose). Incluye versión de la app y del juego, dispositivo/Android, escena, duración, velocidad 1×/2× y gamepad si lo utilizaste. Los resultados correctos también cuentan para la compatibilidad.

Las tareas se priorizan por impacto y reproducción: primero pérdida de progreso o bloqueos, después funciones que impiden avanzar, luego estabilidad/rendimiento y finalmente nuevas capacidades.
