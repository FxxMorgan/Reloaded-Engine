# Procedencia y agradecimientos

Reloaded Engine combina integración Android y cambios desarrollados en este repositorio con un runtime de terceros y adaptaciones basadas en trabajo previo. No debe presentarse como un motor escrito completamente desde cero.

## OpenGMK

- Proyecto: <https://github.com/OpenGMK/OpenGMK>
- Código incluido: `native/vendor/OpenGMK/`.
- Licencia: GNU General Public License, versión 2, según su [LICENCE.md](native/vendor/OpenGMK/LICENCE.md).

OpenGMK aporta el lector de ejecutables GameMaker, el intérprete de GML, la gestión de recursos y la base del renderizado y audio. Este repositorio contiene modificaciones para su integración y compatibilidad en Android. Deben conservarse los avisos originales.

## Klisee / pokemon-reloaded-android

- Proyecto de referencia: <https://github.com/Klisee/pokemon-reloaded-android>
- Referencia local utilizada durante el desarrollo: `KliseePokemonReloadedAndroid`.

Agradecemos a Klisee el concepto general y también el trabajo técnico consultado y adaptado. El análisis de su implementación y de los cambios en su OpenGMK permitió identificar estas correcciones:

| Área | Referencia y relación con este proyecto |
| --- | --- |
| Persistencia de salas | Comprobar la sala saliente después del intercambio en `load_room`; referencia `a79fc2a`. |
| Transparencia de sprites | Comparar RGBA al eliminar el fondo para conservar los contornos negros opacos; referencia `7c0bc92`. |
| Máscaras de colisión | Usar los límites del sprite cuando una máscara está vacía o sin píxeles activos; referencias `e0de882` y `0434cff`. |
| Sprites externos `.gmspr` | Formato, funciones de carga y tolerancia a máscaras incompletas contrastados con su implementación; aquí se añadió un lector separado con validaciones y pruebas. |
| Audio MIDI y rutas | Manejo de `.mid`, normalización de rutas de Windows y síntesis por bloques; referencia adicional `5986012`. |
| Temporización | Restauración de las pausas reales solicitadas por el juego frente al bypass de Android. |

Los identificadores corresponden a la copia de referencia consultada y a su historial del motor. Esta atribución documenta referencias y adaptaciones concretas; no afirma que toda la integración Android proceda de ese repositorio.

Las adaptaciones dentro de OpenGMK forman parte del código del motor distribuido bajo su licencia GPL-2.0. Un agradecimiento al concepto por sí solo omitiría parte de la procedencia técnica.

## Pokémon Reloaded

- Creadores: F3R Productions y el equipo de Pokémon Reloaded.
- Sitio del proyecto: <https://pokemon-reloaded.blogspot.com/>.

El juego se usa para desarrollar y verificar la compatibilidad inicial. Sus ejecutables, partidas, música, imágenes y demás recursos no forman parte de la distribución del launcher. Reloaded Engine es un proyecto independiente.

## TimGM6mb SoundFont

El banco `app/src/main/assets/soundfont.sf2` es TimGM6mb, de **Tim Brechbill (2004) y David Bolton (2010)**, bajo GPL-2.0. Su contenido coincide exactamente con el archivo de [MuseScore](https://github.com/musescore/musescore-old/blob/90c33ef9d87b3f5ff92efd3b07d89eb455fb1fef/mscore/share/sound/TimGM6mb.sf2): blob Git `e5f72e60926a3b62264cb5ad79304f7e204ccac7`.

La atribución y licencia están documentadas por [Debian](https://metadata.ftp-master.debian.org/changelogs/main/t/timgm6mb-soundfont/timgm6mb-soundfont_1.3-5_copyright). El APK incluye `SOUNDFONT-NOTICE.txt` y el texto de GPL-2.0. La conexión de este banco al reproductor MIDI de Android todavía está pendiente.

## Otras dependencias

El proyecto utiliza, entre otras bibliotecas, Kotlin, Jetpack Compose y AndroidX en la aplicación, y crates de Rust declarados en los archivos `Cargo.toml` y `Cargo.lock`. Cada dependencia mantiene su propia licencia. Estos agradecimientos no sustituyen sus avisos de licencia ni los del código incluido en `native/vendor/`.
