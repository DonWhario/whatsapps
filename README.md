# WhatsApps

Cliente de escritorio para Linux que ejecuta **WhatsApp Web** como una app
nativa, con **llamadas y videollamadas** funcionando. Escrito en Rust.

> ⚠️ **Proyecto no oficial.** No está afiliado, asociado ni respaldado por
> WhatsApp LLC ni Meta. "WhatsApp" es una marca de sus respectivos dueños.
> Esta herramienta solo abre el sitio oficial *web.whatsapp.com* en una ventana
> dedicada; no reimplementa ni modifica el servicio.

## ¿Por qué existe?

En Linux no hay app oficial de WhatsApp, y las alternativas basadas en WebView
(WebKitGTK) **no permiten llamadas**: WhatsApp Web restringe las llamadas a
motores Chromium. Este proyecto resuelve eso de forma pragmática: en lugar de
embeber un WebView, **lanza una instancia dedicada de un navegador Chromium**
(Chromium, Chrome, Edge o Brave) en modo aplicación, apuntando a WhatsApp Web,
con perfil propio y persistente. Al ser Chromium real, las llamadas funcionan.

Rust aporta el "envoltorio" nativo: detección del navegador, perfil aislado,
título de ventana propio, icono en la barra superior (bandeja) y un punto único
desde el que arrancar.

## Características

- 📞 Llamadas y videollamadas (motor Chromium).
- 🪟 Ventana con título propio **"WhatsApps"** (vía una mini-extensión que se
  genera en tiempo de ejecución).
- 🔝 Icono en la barra superior (StatusNotifier/AppIndicator) con menú
  *Abrir / Salir*.
- 🔒 Perfil dedicado y persistente (no hay que re-escanear el QR en cada uso).
- 🖥️ Detección automática del navegador: binarios nativos y apps Flatpak.

## Requisitos

- Rust y Cargo (edición 2021).
- Un navegador Chromium: `chromium`, `google-chrome`, `microsoft-edge` o
  `brave-browser`, nativo o como Flatpak.
- Para el icono de bandeja en GNOME: la extensión
  *AppIndicator and KStatusNotifierItem Support*.

## Compilación

~~~bash
git clone <URL-DE-TU-REPO> whatsapps-app
cd whatsapps-app
cargo build --release
~~~

El binario queda en `target/release/whatsapps`.

## Instalación del icono y el lanzador (opcional)

Para que aparezca en el menú de aplicaciones con su propio icono:

~~~bash
# 1) Icono (usa tu propio PNG; aquí un ejemplo con uno en ~/icono.png)
ICONBASE=~/.local/share/icons/hicolor
for s in 512 256 128 64 48 32; do
  mkdir -p "$ICONBASE/${s}x${s}/apps"
  magick ~/icono.png -resize ${s}x${s} "$ICONBASE/${s}x${s}/apps/whatsapps.png"
done

# 2) Entrada de menú (.desktop)
cat > ~/.local/share/applications/whatsapps.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=WhatsApps
Comment=WhatsApp Web como aplicación de escritorio (con llamadas)
Exec=/home/USUARIO/whatsapps-app/target/release/whatsapps
Icon=whatsapps
Terminal=false
Categories=Network;InstantMessaging;Chat;
StartupNotify=true
StartupWMClass=whatsapps
EOF

update-desktop-database ~/.local/share/applications
~~~

> Ajusta la ruta `Exec=` a tu usuario. Usa tu propia imagen para el icono.

## Uso

~~~bash
./target/release/whatsapps
~~~

O ábrelo desde el menú de aplicaciones. La primera vez, escanea el código QR
con tu teléfono (WhatsApp → Ajustes → Dispositivos vinculados). Concede permiso
de micrófono/cámara cuando lo pida para las llamadas.

## Cómo funciona

- **Detección de navegador:** busca binarios Chromium en el `PATH` y, si no hay,
  apps Flatpak conocidas.
- **Modo app:** lanza el navegador con `--app=https://web.whatsapp.com`, un
  `--user-data-dir` dedicado y `--class=whatsapps`.
- **Título:** una extensión Manifest V3 mínima (generada al vuelo) fija el
  título de la ventana en "WhatsApps".
- **Bandeja:** el proceso publica un icono StatusNotifier con
  [`ksni`](https://crates.io/crates/ksni) y queda residente.

## Notas y limitacioness

- **Agrupación en el dock:** con Edge en Flatpak, GNOME agrupa la ventana bajo
  Edge (asocia las ventanas por la identidad del Flatpak). Para una entrada
  totalmente separada, usa un **Chromium nativo**.
- WhatsApp desaconseja clientes no oficiales; este solo carga su web oficial en
  una ventana, pero úsalo con conocimiento de ello.

## Licencia

MIT (ver archivo `LICENSE`).
