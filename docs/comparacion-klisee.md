# Diferencias con la referencia de Klisee

Esta comparación describe Reloaded Engine alpha-0.1 y la [revisión `d0a86a8`](https://github.com/Klisee/pokemon-reloaded-android/tree/d0a86a8f304f89b79ee00d6ff09bcbe789c4ebb6) de pokemon-reloaded-android consultada durante el desarrollo. No representa todos los cambios futuros de ninguno de los proyectos.

Ambos utilizan OpenGMK y ejecutan los datos del juego original. Los dos tienen frontend Kotlin/Compose, integración JNI, importación ZIP y controles táctiles y gamepad. Esas características compartidas no son ventajas exclusivas de Reloaded Engine.

| Área | Reloaded Engine alpha-0.1 | Referencia de Klisee |
| --- | --- | --- |
| Producto | Se presenta como launcher para una futura biblioteca; actualmente mantiene una instalación y un juego probado. | App centrada en Pokémon Reloaded. Su README también anuncia un proyecto posterior llamado Reloaded OpenGMK. |
| Interfaz | Portada de biblioteca, temas claro/oscuro y herramientas del juego recogidas por defecto; diseño propio de esta integración. | Launcher y ajustes específicos para el juego, con personalización de controles y otras funciones. |
| Integración nativa | Workspace con `reloaded-core` y `reloaded-jni`, y una copia de OpenGMK con cambios incluidos en el repositorio. | Integración Android dentro de su árbol de OpenGMK y aplicación Android separada. |
| Sprites externos | Lector `.gmspr` separado, límites de tamaños, recuperación ante errores y pruebas de regresión del formato, transparencia y colisiones. | Implementación y parches de compatibilidad que se consultaron y adaptaron; se reconocen en los créditos. |
| Partidas | Lista de archivos reales, conservación durante la importación y copias ZIP locales/externas verificadas byte a byte. Restaurar desde la interfaz queda pendiente. | Documenta exportación, importación, borrado y movimiento entre ranuras: tiene más funciones en esta área. |
| Presentación y velocidad | Objetivo de 30 pasos/s a 1× y aceleración 2×; se retiró 4× por bloqueos. | Documenta modos de dibujo 30/60/120/sin límite, separados de la simulación. No equivalen a nuestra aceleración 2×. |
| Alcance de distribución | Primera alpha ARM64; validación inicial en un Motorola edge 40 y recopilación pública de experiencias. | Documenta APK ARM64, ARMv7 y x86_64, además de universal. Su alcance de arquitecturas es mayor. |
| Evidencia de esta alpha | Pruebas del núcleo, JNI, sprites y exportación; barrido de 26.237 sprites; límites y siete fallos numéricos heredados publicados. | Sus propias pruebas y correcciones se describen en su repositorio; no hemos realizado un benchmark comparativo. |

## Qué aporta esta entrega

La aportación comprobable es una integración y una interfaz orientadas al launcher, un cargador de sprites con validaciones y pruebas explícitas, y un flujo de exportación de partidas verificado. Las plantillas de reportes permiten recoger datos comparables de dispositivos y gamepads para dirigir las siguientes correcciones.

Esto no demuestra mayor rendimiento, estabilidad o compatibilidad que Klisee: haría falta ejecutar la misma versión del juego, la misma escena y los mismos dispositivos en ambas aplicaciones. Sus funciones más avanzadas de partidas, renderizado y arquitecturas tampoco se anuncian como disponibles aquí.

La procedencia de los parches compartidos está detallada en [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md). No se presentan como invenciones exclusivas los arreglos derivados de su trabajo.
