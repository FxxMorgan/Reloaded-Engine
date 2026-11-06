# Roadmap de Reloaded Engine

Actualizado el **5 de noviembre de 2026**.

Reloaded Engine busca convertirse en un launcher de juegos GameMaker 8 para Android. La alpha pública actual mantiene una instalación y tiene Pokémon Reloaded como primer juego probado. La prioridad principal es usar la misma partida en PC y Android, en ambas direcciones. Después se completarán la pantalla ajustable, los controles contextuales/personalizables y una mayor fluidez de dibujo, antes de ampliar la biblioteca.

Las fases describen prioridades y criterios de salida. No son fechas comprometidas. Un bloqueo de progreso o pérdida de datos confirmado puede adelantar cualquier tarea.

## Experiencia que queremos entregar

1. **Partidas PC ↔ Android:** importar, jugar, guardar, exportar y continuar en el otro dispositivo, empezando por Pokémon Reloaded **1.9.2.2 en ambos lados**.
2. **Pantalla a elección:** conservar la proporción original o estirar el juego hasta ocupar toda la pantalla del móvil.
3. **Controles según la situación:** exploración, combate, Safari, diálogos y menús, mostrando las acciones útiles para cada caso.
4. **Personalización completa:** posición, tamaño, opacidad, forma, diseño e iconos, con perfiles guardados y compartibles.
5. **Más fluidez:** aspirar a 60 FPS de dibujo conservando la velocidad y lógica originales; evaluar 120 Hz después, en dispositivos adecuados.

La implementación de Klisee es una guía funcional y técnica para estos flujos. Se mantendrán los créditos y las licencias de las adaptaciones. Los casos de uso y las pruebas se detallan en [experiencia de juego y compatibilidad](docs/experiencia-y-compatibilidad.md).

## Estado actual

- **Disponible en alpha-0.1:** importación ZIP, controles táctiles y gamepad, temas claro/oscuro, ejecución a 1×/2× y copias de partidas ZIP.
- **Preparado en desarrollo:** corrección de teclas retenidas al limpiar la entrada, con pruebas del estado real de teclado de GameMaker y limpieza en Android. Pendiente de comprobar con mando y dispositivo físicos y de incluirse en un APK posterior.
- **Pendiente confirmado en código:** diálogos de texto/confirmación del kernel, conexión del SoundFont al motor, varias teclas/acciones auxiliares y persistencia completa de la configuración de controles.
- **Por reproducir:** cierres de sesiones largas, degradación al abrir menús, fallos específicos de Pokédex, Galería de la Fama y algunas escenas del juego.

La [revisión de las 63 issues abiertas de Klisee](docs/triage-klisee.md) documenta las fuentes, las coincidencias y lo que ya existe en nuestra implementación. Sus reportes se utilizan como casos de prueba; una coincidencia de síntomas no basta para dar un fallo por confirmado o corregido aquí.

## Fase 1 · Partidas PC ↔ Android y sesiones seguras

| Prioridad | Trabajo | Criterio de comprobación |
| --- | --- | --- |
| P0 | Liberar entradas al perder foco, pausar, desconectar el mando, destruir la superficie o cancelar el joystick. | Las teclas activas se liberan en GameMaker, los taps cancelados no reaparecen y los mismos controles funcionan al volver. Pruebas automatizadas preparadas; falta comprobación física. |
| P0 | Importar y exportar partidas PC ↔ Android en su formato original, con restauración segura. | Cargar una partida real de PC en Android 1.9.2.2, avanzar y guardar, exportarla y volver a cargarla en el juego original de PC 1.9.2.2. Conservar `.savc`, `.data`, `.nfo`, respaldos y los datos asociados que correspondan. |
| P0 | Hacer transaccional la importación y preparar una copia antes de sustituir datos. | Una importación corrupta o interrumpida conserva una instalación utilizable y sus partidas. Incluir `partidas`, `equipos` y `char_custom` cuando existan; distinguir integridad del ZIP de compatibilidad real dentro del juego. |
| P0 | Implementar confirmación y entrada de texto de Android para las funciones pendientes del kernel. | Cancelar, aceptar vacío y escribir texto no bloquean el motor; confirmar una acción siempre corresponde a la elección del usuario. |
| P0 | Reproducir cierres y degradación de rendimiento con logs de nuestra aplicación. | Casos repetibles de cambio de sala, bolsa/equipo y segundo plano; registrar tiempos de cuadro, memoria e instancias antes de elegir un parche. |
| P0 | Verificar coherencia de fechas y zonas horarias entre PC y Android. | Pruebas con copias de partida y reloj correcto, sin modificar el progreso original para ocultar un error. |

Para publicar el siguiente parche: CI requerida aprobada, firma de distribución conservada y prueba en dispositivo de cada flujo modificado. El APK público `alpha-0.1` permanece como referencia; los cambios en `main` no se anuncian como incluidos en él.

## Fase 2 · Pantalla, controles contextuales y personalización

- Ofrecer dos modos de pantalla persistentes: proporción original y estirado a toda la pantalla. Comprobar las coordenadas táctiles y la posición de los controles en ambos.
- Cambiar automáticamente los botones por contexto: exploración, combate, Safari, diálogos, selección de personaje y menús/PC. En Safari mostrar acciones de captura sin la botonera de ataques del combate normal.
- Mantener un perfil manual y un acceso a controles completos si la detección no identifica una escena; al cambiar de perfil liberar las teclas que se ocultan.
- Permitir cambiar forma, diseño e icono de cada botón, con iconos incluidos y PNG personalizados. La apariencia y el área táctil deben ser configurables de forma coherente.
- Añadir una vía de entrada para teclas auxiliares como B, Q y AltGr, botones de ratón y desplazamiento, con accesos que no saturen la pantalla.
- Ofrecer mapeo de gamepad por dispositivo y juego, con ejes, cruceta, gatillos analógicos, reconexión y navegación de menús.
- Guardar distribución, tamaño, opacidad, modo de joystick y apariencia por contexto y juego. Importar y exportar los perfiles de controles por separado de las partidas.
- Revisar Safari, PC/cajas, Pokédex, selección de personajes y mapa con los controles configurados.
- Mantener comprobadas las diagonales y el joystick dinámico que ya existen.

**Criterio de salida:** completar esas escenas con pantalla táctil y al menos dos modelos de gamepad; documentar las asignaciones y conservar los ajustes al reiniciar. Pasar de combate a Safari, diálogo y PC sin perder acciones necesarias, saturar la pantalla ni dejar teclas retenidas. Mantener operativo el modo manual.

## Fase 3 · Fluidez de dibujo, audio y sesiones largas

- Separar la frecuencia de dibujo de los pasos de simulación. Añadir primero un modo de 60 FPS con interpolación de movimientos y cámara, manteniendo la lógica, los temporizadores y el audio a su velocidad original.
- Mantener la aceleración 2× como ajuste independiente del modo de dibujo. No conseguir 60 FPS duplicando los pasos del juego.
- Evaluar 120 FPS después de validar 60, según pantalla y rendimiento real. Mostrar FPS de dibujo y pasos/s como métricas distintas y volver al modo original cuando sea necesario.
- Cargar el SoundFont empaquetado y verificar reproducción MIDI real.
- Recuperar la salida de audio ante cambio de auriculares/Bluetooth/USB y errores de AudioTrack; definir el comportamiento de foco de audio.
- Perfilar el caso de Túnel Gaara y las aperturas repetidas de bolsa/equipo, además de sesiones prolongadas y retorno desde segundo plano.
- Verificar superficies guardadas, Galería de la Fama y sprites de huevos en las vistas correspondientes.
- Investigar las siete comparaciones numéricas heredadas de OpenGMK con la semántica de GameMaker como referencia; mantener visible la auditoría mientras sigan pendientes.

**Criterio de salida:** resultados reproducibles en más de una familia de GPU, una sesión prolongada con métricas conservadas y pruebas de las rutas de audio y gráficos modificadas. A 60 FPS y velocidad 1×, el juego debe conservar el mismo avance lógico que en el modo original: sin duplicar pasos, escrituras, sonidos ni efectos de los eventos de dibujo. Verificar cambios de sala y limpiar la interpolación en las transiciones. No se promete una tasa de FPS universal.

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
