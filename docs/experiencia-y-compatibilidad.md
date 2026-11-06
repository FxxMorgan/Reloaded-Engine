# Experiencia de juego y compatibilidad

Requisitos acordados el **5 de noviembre de 2026**. La referencia inicial para las partidas es **Pokémon Reloaded 1.9.2.2 tanto en PC como en Android**, confirmada por el usuario.

El objetivo es poder continuar la misma partida en los dos dispositivos y jugar en el móvil con una pantalla y unos controles adecuados a cada situación. La implementación de Klisee sirve como guía de comportamiento, arquitectura y casos de prueba, manteniendo la atribución de las adaptaciones.

## 1. Partidas PC ↔ Android: primera prioridad

### Flujo esperado

1. Guardar dentro del juego en PC.
2. Importar sus archivos desde Android y ver la partida correcta en la pantalla de carga.
3. Cargarla, avanzar y guardar desde Android.
4. Exportar los archivos resultantes al PC y continuar en el juego original.
5. Poder repetir el recorrido y conservar una copia anterior para recuperar el progreso.

La transferencia mantendrá el formato original y los bytes de los archivos, sin convertir la partida a un formato exclusivo de Reloaded Engine. El empaquetado ZIP facilita el transporte; el juego debe seguir leyendo sus archivos habituales después de extraerlos.

### Qué hay que conservar

- La partida `.savc`, sus metadatos `.data` y `.nfo` y los respaldos asociados cuando existan.
- La estructura y los archivos de `data/partidas`.
- Los datos de `data/equipos` y `data/char_custom` cuando formen parte del progreso que se transfiere.
- Los nombres, ranuras y relaciones entre archivos. Cualquier cambio de ranura será una acción explícita del usuario.
- Las imágenes u otros archivos de progreso asociados que las pruebas del juego identifiquen; comprobar también la Galería de la Fama.

Antes de sustituir archivos se mostrará qué se importará y se preparará una copia recuperable. Un ZIP incompleto, inválido o una transferencia interrumpida no debe dejar una mezcla de archivos viejos y nuevos. La importación de un paquete de juego tampoco debe borrar el progreso existente.

### Fechas y versiones

La primera prueba usa 1.9.2.2 en ambos lados. La compatibilidad entre versiones diferentes necesita validación independiente. Las fechas y zonas horarias se comprobarán con copias de prueba y relojes correctos; un error de coherencia temporal no se dará por resuelto ocultando el aviso o alterando silenciosamente la partida.

### Pruebas necesarias

| Prueba | Resultado exigido |
| --- | --- |
| Integridad del transporte | Los archivos extraídos conservan sus bytes, nombres y estructura; se incluyen los componentes asociados. |
| PC → Android | El juego de Android carga la partida real de PC y conserva sus datos. |
| Guardado en Android | Después de avanzar y guardar, la partida vuelve a cargar en Android. |
| Android → PC | El juego original de PC carga el guardado producido en Android y refleja ese avance. |
| Varias ranuras | Importar una ranura no sustituye otra sin confirmación; los respaldos siguen asociados a la correcta. |
| Error y recuperación | Cancelar, faltar espacio o interrumpir una importación permite recuperar una copia consistente. |
| Datos complementarios | Los equipos, personajes personalizados y archivos de progreso asociados comprobados siguen disponibles en ambos lados. |

**Estado actual:** existe exportación ZIP de `data/partidas` comprobada byte a byte. Falta implementar la restauración completa y ejecutar el recorrido en el juego original de PC. La integridad del ZIP por sí sola no demuestra compatibilidad de carga en ambos motores.

## 2. Pantalla original o estirada

Ofrecer dos opciones que se conserven entre sesiones:

| Modo | Comportamiento |
| --- | --- |
| Proporción original | El juego mantiene su forma y utiliza bandas cuando la pantalla es más ancha. |
| Toda la pantalla | El juego se estira para llenar el área de juego del móvil, cambiando su proporción. |

El escalado afecta a la imagen y a la conversión de coordenadas táctiles. Hay que comprobar menús, selección en PC/cajas y cualquier interacción de ratón en ambos modos. Los controles deben respetar el área segura, los recortes de cámara y los gestos del sistema.

**Estado actual:** el renderer fuerza la proporción cuando dibuja sobre la superficie Android. La preferencia y el cambio de modo están pendientes.

## 3. Controles por contexto

Los controles visibles deben seguir la situación del juego. El perfil que el usuario está editando y el perfil activo durante la partida se gestionarán por separado.

| Situación | Comportamiento esperado |
| --- | --- |
| Exploración | Movimiento y accesos útiles para el mundo. |
| Combate normal | Ataques, cambio de Pokémon y acciones de combate. |
| Safari | Acciones de captura y objetos correspondientes, sin llenar la pantalla con ataques del combate normal. |
| Diálogos y opciones | Avanzar, responder, aceptar o cancelar. |
| PC, mochila y Pokédex | Navegación, selección y las acciones específicas de cada menú. |
| Selección de personaje | Flechas y confirmación disponibles y estables. |

La detección se basará en información del juego expuesta por su perfil de compatibilidad. No dependerá únicamente de un identificador de sala que pueda cambiar entre versiones.

Al cambiar de perfil se liberarán las teclas de controles que se ocultan. Una escena no identificada conservará un perfil utilizable y ofrecerá acceso manual a los controles completos; el jugador no debe quedar atrapado por una detección incorrecta.

**Comprobación:** recorrer exploración → combate → Safari → menú → diálogo y volver, también con gamepad conectado/desconectado. Revisar especialmente las transiciones asociadas a los reportes de desaparición de botones.

## 4. Personalización real

El editor debe permitir:

- Mover y redimensionar cada control y ajustar su opacidad.
- Cambiar forma, diseño e icono; seleccionar arte incluido o importar PNG propios.
- Personalizar joystick y botones por contexto, conservando una base común cuando se desee.
- Mantener coherentes el dibujo del botón y su zona pulsable.
- Guardar los cambios entre reinicios y adaptarlos a distintas resoluciones y modos de pantalla.
- Exportar/importar perfiles y recuperar los valores por defecto de un control o un perfil.
- Configurar las asignaciones de gamepad por dispositivo y contexto.

Los perfiles de controles se transportarán por separado de las partidas. Los recursos gráficos incorporados deben tener una procedencia y licencia documentadas. La pantalla principal seguirá siendo sobria, sin emojis ni información técnica innecesaria.

**Estado actual:** hay edición de posición, escala y opacidad, pero faltan la persistencia completa, los diseños/iconos intercambiables y los perfiles automáticos.

## 5. Más FPS de dibujo conservando la velocidad del juego

El modo actual 2× aumenta los pasos del juego. El objetivo de 60 FPS es aumentar la fluidez de imagen manteniendo la velocidad normal.

La implementación debe separar:

- **Simulación:** pasos originales del juego, lógica, temporizadores y avance del progreso.
- **Presentación:** dibujos por segundo e interpolación de posiciones/cámara entre pasos.
- **Aceleración:** un ajuste independiente para quien elija jugar a 2×.

Se validará primero 60 FPS; 120 FPS será una opción posterior según pantalla y capacidad del dispositivo. Al cambiar de sala se renovarán los estados de interpolación para evitar saltos o imágenes mezcladas.

Los eventos Draw de GameMaker pueden producir efectos secundarios. Los cuadros intermedios no deben duplicar lógica, escrituras de archivos, sonidos, consumo de aleatoriedad o cambios de sala. La implementación de referencia tiene una ruta `render_only`; esa separación debe estudiarse y probarse antes de trasladarla a nuestro bucle JNI.

**Comprobación:** comparar el mismo intervalo a velocidad 1× en el modo original y en el de 60 FPS. El número de pasos y el avance del juego deben mantenerse equivalentes mientras mejora la fluidez del dibujo. Medir FPS de presentación y pasos/s por separado, incluyendo combate, cambios de sala y segundo plano.

## Referencias de Klisee consultadas

Revisión `d0a86a8f304f89b79ee00d6ff09bcbe789c4ebb6`:

- [Video.kt](https://github.com/Klisee/pokemon-reloaded-android/blob/d0a86a8f304f89b79ee00d6ff09bcbe789c4ebb6/android/app/src/main/java/com/klisee/reloaded/Video.kt): modos de proporción y frecuencia de presentación.
- [TouchLayouts.kt](https://github.com/Klisee/pokemon-reloaded-android/blob/d0a86a8f304f89b79ee00d6ff09bcbe789c4ebb6/android/app/src/main/java/com/klisee/reloaded/TouchLayouts.kt): perfiles por contexto y catálogo de controles.
- [TouchSprites.kt](https://github.com/Klisee/pokemon-reloaded-android/blob/d0a86a8f304f89b79ee00d6ff09bcbe789c4ebb6/android/app/src/main/java/com/klisee/reloaded/TouchSprites.kt): apariencia e imágenes personalizadas.
- [GameRepository.kt](https://github.com/Klisee/pokemon-reloaded-android/blob/d0a86a8f304f89b79ee00d6ff09bcbe789c4ebb6/android/app/src/main/java/com/klisee/reloaded/GameRepository.kt): transferencia de partidas y carpetas de datos del jugador.
- [game.rs](https://github.com/Klisee/pokemon-reloaded-android/blob/d0a86a8f304f89b79ee00d6ff09bcbe789c4ebb6/src/OpenGMK/gm8emulator/src/game.rs): separación del renderizado intermedio.

La procedencia de las adaptaciones permanece en [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md). La prioridad de entrega está en [ROADMAP.md](../ROADMAP.md).
