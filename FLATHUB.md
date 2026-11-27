# Publicar Strand en Flathub

Checklist de lo que falta para publicar, en orden.

## 1. Instalar herramientas (local, no incluido aquí)

```bash
sudo apt install flatpak flatpak-builder
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install flathub org.gnome.Platform//47 org.gnome.Sdk//47
flatpak install flathub org.freedesktop.Sdk.Extension.rust-stable org.freedesktop.Sdk.Extension.node20
```

Ajustar la versión de runtime (`47`) si para entonces Flathub recomienda otra.

## 2. Generar las fuentes offline

Flathub compila sin red, así que hay que vendorizar las dependencias de Cargo y de node. Ver los comentarios en `io.github.cgm98.strand.yml` con los comandos exactos (`flatpak-cargo-generator.py` y `flatpak-node-generator.py`, de [flatpak/flatpak-builder-tools](https://github.com/flatpak/flatpak-builder-tools)). El resultado de cada script hay que fusionarlo dentro del array `sources:` del manifiesto.

## 3. Captura de pantalla

El `metainfo.xml` necesita al menos un `<screenshot>` con una URL estable (por ejemplo, subida al propio repo de GitHub y referenciada por su URL raw, o a un hosting dedicado). Agregar el bloque `<screenshots>` antes de `<releases>`.

## 4. Build y validación local

```bash
flatpak-builder --force-clean --user --install build-dir io.github.cgm98.strand.yml
flatpak run io.github.cgm98.strand
appstreamcli validate src-tauri/packaging/io.github.cgm98.strand.metainfo.xml
desktop-file-validate src-tauri/packaging/io.github.cgm98.strand.desktop
```

Iterar hasta que todo pase limpio.

## 5. Tag de release

Flathub compila desde un `tag` real del repo, no desde un directorio local. Crear un release (`git tag v0.1.0 && git push --tags`) y cambiar el `source` del manifiesto a `type: git` apuntando a ese tag + commit, como se indica en los comentarios del manifiesto.

## 6. Submission

1. Hacer fork de [flathub/flathub](https://github.com/flathub/flathub).
2. Crear una rama con un único archivo: `io.github.cgm98.strand.yml` (el manifiesto final, con sources resueltas).
3. Abrir el PR siguiendo la [guía oficial de submission](https://docs.flathub.org/docs/for-app-authors/submission). Un bot crea automáticamente el repo `flathub/io.github.cgm98.strand` y un build de prueba.
4. Responder a la revisión humana de Flathub (suelen pedir ajustes de permisos/sandbox o metadata).
5. Una vez aprobado, hacer merge — eso dispara el primer build publicado en Flathub.

## Notas de permisos (ya resueltas en el manifiesto)

- Sin `--share=network`: Strand es 100% local (CSP `connect-src 'self'`).
- Sin `--filesystem` extra: los datos de la app usan `app_data_dir()`, que ya cae dentro del sandbox de Flatpak.
- El selector de archivos usa el portal de `xdg-desktop-portal`, no necesita permisos de filesystem.
