// whatsapps — cliente de escritorio Linux para WhatsApp Web.
//
// Arranca una instancia DEDICADA de un navegador Chromium (Chrome/Chromium/
// Edge/Brave) en modo `--app` apuntando a WhatsApp Web, con perfil propio y
// persistente. Al ser Chromium real, las llamadas y videollamadas funcionan.
//
// Además publica un icono en la barra superior de GNOME (StatusNotifier /
// AppIndicator) mientras la app está abierta, con un menú para reabrir o salir.

use std::env;
use std::path::{Path, PathBuf};
use std::process::{exit, Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const APP_URL: &str = "https://web.whatsapp.com";
const WM_CLASS: &str = "whatsapps"; // identidad/nombra la ventana en el escritorio
const WINDOW_TITLE: &str = "WhatsApps"; // título que forzamos en la ventana
const ICON_NAME: &str = "whatsapps"; // icono instalado en el tema hicolor

// Mini-extensión (Manifest V3) que fija el título de la ventana en "WhatsApps".
const EXT_MANIFEST: &str = r#"{
  "manifest_version": 3,
  "name": "WhatsApps Title",
  "version": "1.0",
  "description": "Fija el titulo de la ventana en 'WhatsApps'.",
  "content_scripts": [
    {
      "matches": ["https://web.whatsapp.com/*"],
      "js": ["content.js"],
      "run_at": "document_start"
    }
  ]
}
"#;

const EXT_CONTENT_TEMPLATE: &str = r#"(function () {
  var BASE = "__TITLE__";
  function desired() {
    var m = document.title.match(/\((\d+)\)/) || document.title.match(/(\d+)/);
    return m ? BASE + " (" + m[1] + ")" : BASE;
  }
  function apply() {
    var want = desired();
    if (document.title !== want) document.title = want;
  }
  function start() {
    var t = document.querySelector("title");
    if (t) new MutationObserver(apply).observe(t, { childList: true });
    apply();
    setInterval(apply, 1000);
  }
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", start);
  } else {
    start();
  }
})();
"#;

/// Un navegador Chromium detectado y cómo invocarlo.
#[derive(Clone)]
enum Browser {
    Native { name: String, path: String },
    Flatpak { name: String, app_id: String },
}

fn home() -> PathBuf {
    PathBuf::from(env::var("HOME").unwrap_or_else(|_| "/root".into()))
}

/// Escribe la extensión de título en `dir` (creándolo si no existe).
fn write_title_extension(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(dir.join("manifest.json"), EXT_MANIFEST)?;
    let content = EXT_CONTENT_TEMPLATE.replace("__TITLE__", WINDOW_TITLE);
    std::fs::write(dir.join("content.js"), content)?;
    Ok(())
}

/// Busca un navegador Chromium, primero nativo y luego Flatpak.
fn detect_browser() -> Option<Browser> {
    let native_candidates = [
        ("Chromium", "chromium"),
        ("Chromium", "chromium-browser"),
        ("Google Chrome", "google-chrome"),
        ("Google Chrome", "google-chrome-stable"),
        ("Brave", "brave-browser"),
        ("Microsoft Edge", "microsoft-edge"),
        ("Microsoft Edge", "microsoft-edge-stable"),
        ("Vivaldi", "vivaldi"),
    ];
    for (name, bin) in native_candidates {
        if let Some(path) = which(bin) {
            return Some(Browser::Native {
                name: name.to_string(),
                path,
            });
        }
    }

    let flatpak_candidates = [
        ("Chromium", "org.chromium.Chromium"),
        ("Google Chrome", "com.google.Chrome"),
        ("Brave", "com.brave.Browser"),
        ("Microsoft Edge", "com.microsoft.Edge"),
        ("Vivaldi", "com.vivaldi.Vivaldi"),
    ];
    if which("flatpak").is_some() {
        let installed = flatpak_installed_apps();
        for (name, app_id) in flatpak_candidates {
            if installed.iter().any(|a| a == app_id) {
                return Some(Browser::Flatpak {
                    name: name.to_string(),
                    app_id: app_id.to_string(),
                });
            }
        }
    }

    None
}

/// Equivalente mínimo a `which`: busca `bin` en el PATH.
fn which(bin: &str) -> Option<String> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let full = dir.join(bin);
        if full.is_file() {
            return Some(full.to_string_lossy().into_owned());
        }
    }
    None
}

/// Lista los IDs de apps Flatpak instaladas.
fn flatpak_installed_apps() -> Vec<String> {
    let out = Command::new("flatpak")
        .args(["list", "--app", "--columns=application"])
        .output();
    match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

/// Construye el comando para lanzar el navegador en modo app dedicado.
fn build_command(browser: &Browser) -> Command {
    let common: Vec<String> = vec![
        format!("--app={APP_URL}"),
        format!("--class={WM_CLASS}"),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
        // Silencia los avisos de arranque (extensiones en modo desarrollador,
        // banner de flags no soportados).
        "--test-type".into(),
        "--ozone-platform=x11".into(),
        "--ozone-platform=x11".into(),
    ];

    match browser {
        Browser::Native { path, .. } => {
            let base = home().join(".local/share/whatsapps");
            let profile = base.join("profile");
            let ext = base.join("extension");
            let _ = std::fs::create_dir_all(&profile);
            let _ = write_title_extension(&ext);
            let mut c = Command::new(path);
            c.arg(format!("--user-data-dir={}", profile.display()));
            c.arg(format!("--load-extension={}", ext.display()));
            c.args(&common);
            c
        }
        Browser::Flatpak { app_id, .. } => {
            // Perfil y extensión dentro del árbol accesible por el sandbox.
            let base = home().join(".var/app").join(app_id).join("data");
            let profile = base.join("whatsapps-profile");
            let ext = base.join("whatsapps-extension");
            let _ = std::fs::create_dir_all(&profile);
            let _ = write_title_extension(&ext);
            let mut c = Command::new("flatpak");
            c.args(["run", app_id]);
            c.arg(format!("--user-data-dir={}", profile.display()));
            c.arg(format!("--load-extension={}", ext.display()));
            c.args(&common);
            c
        }
    }
}

/// Lanza el navegador y guarda el hijo en `slot` (para poder cerrarlo luego).
fn launch_browser(browser: &Browser, slot: &Arc<Mutex<Option<Child>>>) {
    match build_command(browser).spawn() {
        Ok(child) => {
            if let Ok(mut g) = slot.lock() {
                // Solo guardamos el primer proceso "real"; los relanzamientos
                // en una instancia ya abierta terminan enseguida.
                if g.is_none() {
                    *g = Some(child);
                }
            }
        }
        Err(e) => eprintln!("whatsapps: no se pudo lanzar el navegador: {e}"),
    }
}

/// Icono de bandeja en la barra superior.
struct WhatsAppsTray {
    browser: Browser,
    child: Arc<Mutex<Option<Child>>>,
}

impl ksni::Tray for WhatsAppsTray {
    fn id(&self) -> String {
        "whatsapps".into()
    }
    fn title(&self) -> String {
        WINDOW_TITLE.into()
    }
    fn icon_name(&self) -> String {
        ICON_NAME.into()
    }
    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: WINDOW_TITLE.into(),
            description: "WhatsApp Web".into(),
            icon_name: ICON_NAME.into(),
            icon_pixmap: Vec::new(),
        }
    }
    // Clic izquierdo sobre el icono: reabre/enfoca la ventana.
    fn activate(&mut self, _x: i32, _y: i32) {
        launch_browser(&self.browser, &self.child);
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::{StandardItem, MenuItem};
        vec![
            StandardItem {
                label: "Abrir WhatsApps".into(),
                icon_name: ICON_NAME.into(),
                activate: Box::new(|this: &mut Self| {
                    launch_browser(&this.browser, &this.child);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Salir".into(),
                activate: Box::new(|this: &mut Self| {
                    if let Ok(mut g) = this.child.lock() {
                        if let Some(c) = g.as_mut() {
                            let _ = c.kill();
                        }
                    }
                    exit(0);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

fn main() {
    let browser = match detect_browser() {
        Some(b) => b,
        None => {
            eprintln!("whatsapps: no se encontró ningún navegador Chromium.");
            eprintln!("Instala uno (Chromium, Chrome, Edge o Brave), nativo o Flatpak.");
            exit(1);
        }
    };

    let child: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));

    println!("whatsapps: abriendo WhatsApp Web…");
    launch_browser(&browser, &child);

    // Publica el icono en la barra superior y queda residente.
    let tray = WhatsAppsTray {
        browser: browser.clone(),
        child: child.clone(),
    };
    let service = ksni::TrayService::new(tray);
    service.spawn();

    // El proceso vive hasta que el usuario elija "Salir" desde el icono.
    loop {
        std::thread::sleep(Duration::from_secs(3600));
    }
}
