# Compilación, firma y publicación

El repositorio distribuye la alpha como **GitHub prerelease pública**. La descarga contiene un APK ARM64 firmado, el código fuente correspondiente, el certificado público de firma y `SHA256SUMS`. Los archivos del juego y las partidas no se publican.

## Integración continua

El workflow `Android CI and releases` se ejecuta en pushes a `main`, pull requests y tags `alpha-*`. También se puede iniciar desde Actions.

La verificación bloqueante cubre:

- Núcleo y puente JNI en Rust.
- Pruebas de OpenGMK salvo siete comparaciones numéricas heredadas documentadas.
- Tests JVM de la aplicación y lint de Android.

La suite completa de OpenGMK se ejecuta como auditoría adicional visible. Los fallos conocidos se mantienen en el informe; no se presentan como pruebas aprobadas. Los informes se adjuntan a cada ejecución.

## Publicar una alpha

1. Actualiza `versionName` y aumenta `versionCode` en `app/build.gradle.kts`.
2. Añade las notas en `docs/<versionName>.md`.
3. Comprueba los cambios y publica un tag anotado que coincida con `versionName`:

```bash
git tag -a alpha-0.2 -m "Reloaded Engine alpha-0.2"
git push origin alpha-0.2
```

Tras superar la verificación, Actions instala Rust 1.99.1, JDK 21 y NDK 28.2.13676358, compila ARM64, firma y verifica el APK, prepara el código fuente y publica la prerelease. Un tag que no coincida con la versión se rechaza. El workflow no sobrescribe una release que ya sea pública.

## Firma

Los secretos del repositorio utilizados únicamente por el job de publicación son:

| Secreto | Contenido |
| --- | --- |
| `ANDROID_KEYSTORE_BASE64` | Keystore de distribución codificado en base64 |
| `ANDROID_KEYSTORE_PASSWORD` | Contraseña del keystore |
| `ANDROID_KEY_ALIAS` | Alias de la clave |
| `ANDROID_KEY_PASSWORD` | Contraseña de la clave |

La clave privada se conserva fuera del repositorio y tiene una copia cifrada como secreto de Actions. No debe reemplazarse para las actualizaciones de la misma aplicación.

Para firmar localmente, exporta estas variables sin incorporarlas a archivos versionados:

```text
ANDROID_HOME
ANDROID_KEYSTORE_PATH
ANDROID_KEYSTORE_PASSWORD
ANDROID_KEY_ALIAS
ANDROID_KEY_PASSWORD
```

Después de compilar `assembleRelease`:

```bash
./scripts/sign-apk.sh \
  app/build/outputs/apk/release/app-release-unsigned.apk \
  build/release/Reloaded-Engine-alpha-0.1-arm64.apk
```

El script comprueba la alineación del APK y verifica su firma. Solo el certificado y su huella son públicos. El APK de desarrollo usa otra firma; no es una actualización directa de la versión de distribución.
