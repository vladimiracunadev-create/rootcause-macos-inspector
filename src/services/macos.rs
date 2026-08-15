//! Adaptador de sistema para macOS.
//!
//! Todo lo que implica hablar con el sistema operativo pasa por aquí: ejecutar
//! utilidades nativas (`launchctl`, `spctl`, `csrutil`, `lsof`, `arp`,
//! `codesign`, `log`), leer `sysctl` y lanzar notificaciones.
//!
//! Reglas de la casa:
//!
//! * **Solo lectura por defecto.** Las únicas funciones que modifican estado son
//!   [`terminate_process`] y [`reveal_in_finder`], y ambas se auditan arriba.
//! * **Nada de `sudo` implícito.** Si un dato necesita privilegios y no los hay,
//!   se devuelve el error tal cual para que la UI lo explique en vez de simular
//!   que el dato no existe.
//! * **Fallo suave.** Una utilidad ausente o sin permiso nunca debe tumbar una
//!   captura completa: quien llama decide el respaldo.

use crate::models::CodeSignature;
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Ejecuta un binario y devuelve su `stdout` como texto.
///
/// Devuelve error si el proceso no arranca o si termina con código distinto de
/// cero; en ese caso el mensaje incluye `stderr`, que suele ser lo único útil
/// cuando falta un permiso.
pub fn run_capture(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("No se pudo ejecutar {program}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        return Err(anyhow!("{program} devolvió error: {detail}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Igual que [`run_capture`] pero une `stdout` y `stderr` y NO falla por código
/// de salida. Necesario para utilidades que escriben el dato útil en `stderr`
/// (`codesign`) o que salen con código ≠ 0 en estados normales (`spctl`).
pub fn run_combined(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("No se pudo ejecutar {program}"))?;

    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.trim().is_empty() {
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&stderr);
    }
    Ok(text)
}

/// `true` si el binario existe y es ejecutable en el `PATH` o en la ruta dada.
pub fn command_exists(program: &str) -> bool {
    if program.starts_with('/') {
        return Path::new(program).is_file();
    }
    Command::new("/usr/bin/which")
        .arg(program)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Lee una clave de `sysctl` (p. ej. `hw.model`).
pub fn sysctl(key: &str) -> Option<String> {
    run_capture("/usr/sbin/sysctl", &["-n", key])
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

/// Versión de macOS legible (`26.3.1`).
pub fn product_version() -> String {
    run_capture("/usr/bin/sw_vers", &["-productVersion"])
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
}

/// Nombre del producto (`macOS`).
pub fn product_name() -> String {
    run_capture("/usr/bin/sw_vers", &["-productName"])
        .map(|value| value.trim().to_owned())
        .unwrap_or_else(|_| "macOS".to_owned())
}

/// Build del sistema (`25D2128`).
pub fn build_version() -> String {
    run_capture("/usr/bin/sw_vers", &["-buildVersion"])
        .map(|value| value.trim().to_owned())
        .unwrap_or_default()
}

/// Usuario de la consola actual.
pub fn current_user() -> String {
    std::env::var("USER").unwrap_or_else(|_| {
        run_capture("/usr/bin/id", &["-un"])
            .map(|value| value.trim().to_owned())
            .unwrap_or_default()
    })
}

/// UID efectivo del proceso actual (necesario para el dominio `gui/<uid>`).
pub fn current_uid() -> u32 {
    run_capture("/usr/bin/id", &["-u"])
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(501)
}

/// `true` si el proceso corre como root.
pub fn is_root() -> bool {
    current_uid() == 0
}

/// Utilidades del sistema de las que depende la captura.
const REQUIRED_TOOLS: &[(&str, &str)] = &[
    ("/usr/sbin/lsof", "Conexiones por proceso"),
    ("/usr/sbin/spctl", "Estado de Gatekeeper"),
    ("/usr/bin/csrutil", "Estado de SIP"),
    ("/usr/bin/fdesetup", "Estado de FileVault"),
    ("/usr/bin/codesign", "Firma de código"),
    ("/usr/sbin/arp", "Vecinos de red"),
    ("/bin/launchctl", "Servicios de launchd"),
];

/// Contexto de ejecución: quién corre RootCause y con qué herramientas cuenta.
///
/// Importa para interpretar los resultados: sin root, `lsof` solo ve los
/// sockets del propio usuario, y sin `codesign` no hay verificación de firma.
/// Es preferible declararlo que dejar que el usuario asuma cobertura total.
#[derive(Debug, Clone)]
pub struct EnvironmentReport {
    pub user: String,
    pub uid: u32,
    pub is_root: bool,
    /// `(ruta, descripción, disponible)` de cada utilidad requerida.
    pub tools: Vec<(String, String, bool)>,
}

impl EnvironmentReport {
    /// Herramientas que faltan, si las hay.
    pub fn missing_tools(&self) -> Vec<&str> {
        self.tools
            .iter()
            .filter(|(_, _, available)| !available)
            .map(|(path, _, _)| path.as_str())
            .collect()
    }
}

/// Recoge el contexto de ejecución actual.
pub fn environment() -> EnvironmentReport {
    EnvironmentReport {
        user: current_user(),
        uid: current_uid(),
        is_root: is_root(),
        tools: REQUIRED_TOOLS
            .iter()
            .map(|(path, description)| {
                (
                    (*path).to_owned(),
                    (*description).to_owned(),
                    command_exists(path),
                )
            })
            .collect(),
    }
}

// ── Procesos ────────────────────────────────────────────────────────────────

/// Datos que solo `ps` conoce bien en macOS: usuario propietario y línea de
/// comandos completa. Se recogen en una sola llamada por captura porque lanzar
/// `ps` una vez por PID sería absurdamente caro.
#[derive(Debug, Clone, Default)]
pub struct ProcessDetail {
    pub user: String,
    pub command_line: String,
}

/// Tabla `pid → (usuario, línea de comandos)` de todos los procesos visibles.
///
/// Sin root, `ps -ax` lista los procesos de todos los usuarios pero la línea de
/// comandos de procesos ajenos puede aparecer recortada; se acepta tal cual y se
/// documenta como limitación en vez de pedir privilegios.
pub fn process_details() -> HashMap<u32, ProcessDetail> {
    let mut table = HashMap::new();
    let Ok(output) = run_capture("/bin/ps", &["-axo", "pid=,user=,command="]) else {
        return table;
    };

    for line in output.lines() {
        let line = line.trim_start();
        let Some((pid_text, rest)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let Ok(pid) = pid_text.trim().parse::<u32>() else {
            continue;
        };
        let rest = rest.trim_start();
        let Some((user, command)) = rest.split_once(char::is_whitespace) else {
            continue;
        };
        table.insert(
            pid,
            ProcessDetail {
                user: user.trim().to_owned(),
                command_line: command.trim().to_owned(),
            },
        );
    }
    table
}

/// Envía `SIGTERM` a un proceso. No escala a `SIGKILL`: si el proceso ignora la
/// señal, es información útil, no algo que RootCause deba forzar por su cuenta.
pub fn terminate_process(pid: u32) -> Result<String> {
    let output = Command::new("/bin/kill")
        .args(["-TERM", &pid.to_string()])
        .output()
        .context("No se pudo invocar kill")?;

    if output.status.success() {
        Ok(format!("Señal TERM enviada al PID {pid}"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        Err(anyhow!(
            "No se pudo finalizar el PID {pid}: {}",
            if stderr.is_empty() {
                "permiso denegado o el proceso ya no existe".to_owned()
            } else {
                stderr
            }
        ))
    }
}

/// Abre el Finder con el archivo seleccionado. Es la acción "segura" que
/// RootCause ofrece sobre una persistencia sospechosa: mostrarla, no borrarla.
pub fn reveal_in_finder(path: &str) -> Result<String> {
    Command::new("/usr/bin/open")
        .args(["-R", path])
        .status()
        .with_context(|| format!("No se pudo revelar {path} en Finder"))?;
    Ok(format!("Revelado en Finder: {path}"))
}

/// Notificación del sistema mediante `osascript`.
///
/// Puede fallar silenciosamente si el usuario deshabilitó las notificaciones de
/// la app: es una comodidad, nunca el canal principal de una alerta.
pub fn notify(title: &str, message: &str) {
    let safe_title = title.replace('"', "'");
    let safe_message = message.replace('"', "'");
    let script = format!("display notification \"{safe_message}\" with title \"{safe_title}\"");
    let _ = Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .output();
}

// ── Firma de código ─────────────────────────────────────────────────────────

/// Clasifica la firma de código de un binario usando `codesign -dvv`.
///
/// `codesign` escribe el detalle en `stderr`, así que se usa [`run_combined`].
/// La clasificación es deliberadamente conservadora: cualquier cosa que no se
/// entienda queda como [`CodeSignature::Unknown`] en vez de asumir confianza.
pub fn code_signature(path: &str) -> CodeSignature {
    if path.trim().is_empty() || !Path::new(path).exists() {
        return CodeSignature::Unknown;
    }

    let Ok(output) = run_combined("/usr/bin/codesign", &["-dvv", path]) else {
        return CodeSignature::Unknown;
    };
    classify_codesign_output(&output)
}

/// Interpreta la salida de `codesign -dvv`. Separada del proceso para poder
/// probarla sin tocar el sistema.
pub fn classify_codesign_output(output: &str) -> CodeSignature {
    let lower = output.to_ascii_lowercase();

    if lower.contains("code object is not signed at all") {
        return CodeSignature::Unsigned;
    }
    if lower.contains("authority=software signing")
        || lower.contains("authority=apple code signing certification authority")
        || lower.contains("authority=apple root ca")
    {
        return CodeSignature::Apple;
    }
    if lower.contains("authority=developer id application")
        || lower.contains("authority=apple mac os application signing")
        || lower.contains("authority=3rd party mac developer application")
    {
        return CodeSignature::DeveloperId;
    }
    if lower.contains("signature=adhoc") || lower.contains("flags=0x2(adhoc)") {
        return CodeSignature::AdHoc;
    }
    if lower.contains("signature=none") {
        return CodeSignature::Unsigned;
    }
    CodeSignature::Unknown
}

// ── Red ─────────────────────────────────────────────────────────────────────

/// Salida de `lsof -i` en **modo campo** (`-F`), base del tab Conexiones.
///
/// El formato tabular de `lsof` se rompe con nombres de proceso que llevan
/// espacios (`Google Chrome`), así que se pide la salida por campos: una línea
/// por dato, prefijada por su letra. Es fea de leer y trivial de parsear sin
/// ambigüedad.
///
/// Sin root solo se ven los sockets de los procesos del usuario actual: es una
/// limitación real que la UI declara en vez de disimular.
pub fn lsof_connections() -> Result<String> {
    const ARGS: &[&str] = &["-i", "-n", "-P", "-FpcLftPnT"];
    run_capture("/usr/sbin/lsof", ARGS).or_else(|_| run_capture("/usr/bin/lsof", ARGS))
}

/// Tabla de vecinos ARP del segmento local.
pub fn arp_table() -> Result<String> {
    run_capture("/usr/sbin/arp", &["-a", "-n"])
}

/// Interfaz y puerta de enlace de la ruta por defecto, vía `route -n get default`.
pub fn default_route() -> (String, String) {
    let Ok(output) = run_capture("/sbin/route", &["-n", "get", "default"]) else {
        return (String::new(), String::new());
    };

    let mut interface = String::new();
    let mut gateway = String::new();
    for line in output.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("interface:") {
            interface = value.trim().to_owned();
        } else if let Some(value) = line.strip_prefix("gateway:") {
            gateway = value.trim().to_owned();
        }
    }
    (interface, gateway)
}

/// IPv4 y MAC de una interfaz, leídas de `ifconfig`.
pub fn interface_addresses(interface: &str) -> (String, String) {
    if interface.is_empty() {
        return (String::new(), String::new());
    }
    let Ok(output) = run_capture("/sbin/ifconfig", &[interface]) else {
        return (String::new(), String::new());
    };

    let mut ip = String::new();
    let mut mac = String::new();
    for line in output.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("inet ") {
            if ip.is_empty() {
                ip = rest
                    .split_whitespace()
                    .next()
                    .unwrap_or_default()
                    .to_owned();
            }
        } else if let Some(rest) = line.strip_prefix("ether ") {
            mac = rest
                .split_whitespace()
                .next()
                .unwrap_or_default()
                .to_owned();
        }
    }
    (ip, mac)
}

/// Barrido de descubrimiento del segmento `/24`: hace ping a cada host para que
/// el sistema rellene su tabla ARP. Ruidoso a propósito y solo bajo demanda.
///
/// Usa un único `ping -c 1 -t 1` por host en serie con timeout corto: más lento
/// que un escáner dedicado, pero sin dependencias y sin levantar sockets raw.
pub fn discovery_sweep(subnet_prefix: &str) {
    if subnet_prefix.is_empty() {
        return;
    }
    for host in 1..=254 {
        let target = format!("{subnet_prefix}.{host}");
        let _ = Command::new("/sbin/ping")
            .args(["-c", "1", "-t", "1", "-q", &target])
            .output();
    }
}

/// Resuelve el nombre de un host por DNS inverso, si responde rápido.
pub fn reverse_dns(ip: &str) -> String {
    run_capture(
        "/usr/bin/dscacheutil",
        &["-q", "host", "-a", "ip_address", ip],
    )
    .ok()
    .and_then(|output| {
        output
            .lines()
            .find_map(|line| line.trim().strip_prefix("name: ").map(str::to_owned))
    })
    .unwrap_or_default()
}

// ── Log unificado ───────────────────────────────────────────────────────────

/// Eventos recientes de seguridad del log unificado.
///
/// `log show` es caro (segundos) y por eso NO se llama en cada captura: la GUI
/// y el CLI lo invocan bajo demanda. El predicado se acota a los subsistemas
/// que explican decisiones de Gatekeeper, XProtect, TCC y escaladas de
/// privilegio.
pub fn security_log_events(minutes: u32, limit: usize) -> Result<Vec<(String, String, String)>> {
    let last = format!("{minutes}m");
    let predicate = "process == \"syspolicyd\" \
                     OR process == \"XProtect\" \
                     OR process == \"XprotectService\" \
                     OR process == \"tccd\" \
                     OR process == \"sudo\" \
                     OR process == \"amfid\"";

    let output = run_capture(
        "/usr/bin/log",
        &[
            "show",
            "--last",
            &last,
            "--style",
            "compact",
            "--predicate",
            predicate,
        ],
    )?;

    let mut events = Vec::new();
    for line in output.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let timestamp = parts.next().unwrap_or_default().to_owned();
        // Formato compacto: <hora> <tipo> <actividad> <pid> <proceso>: <mensaje>
        let rest: Vec<&str> = parts.collect();
        let joined = rest.join(" ");
        let (provider, message) = match joined.split_once(": ") {
            Some((head, tail)) => (
                head.split_whitespace().last().unwrap_or("log").to_owned(),
                tail.trim().to_owned(),
            ),
            None => ("log".to_owned(), joined),
        };
        events.push((timestamp, provider, message));
        if events.len() >= limit {
            break;
        }
    }
    Ok(events)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clasifica_binario_sin_firma() {
        let output = "test-binary: code object is not signed at all";
        assert_eq!(classify_codesign_output(output), CodeSignature::Unsigned);
    }

    #[test]
    fn clasifica_binario_de_apple() {
        let output = "Executable=/bin/ls\nAuthority=Software Signing\nAuthority=Apple Code Signing Certification Authority";
        assert_eq!(classify_codesign_output(output), CodeSignature::Apple);
    }

    #[test]
    fn clasifica_developer_id() {
        let output = "Executable=/Applications/Foo.app\nAuthority=Developer ID Application: Acme Inc (ABCDE12345)";
        assert_eq!(classify_codesign_output(output), CodeSignature::DeveloperId);
    }

    #[test]
    fn clasifica_adhoc() {
        let output = "Executable=/tmp/x\nSignature=adhoc";
        assert_eq!(classify_codesign_output(output), CodeSignature::AdHoc);
    }

    #[test]
    fn salida_desconocida_no_asume_confianza() {
        assert_eq!(classify_codesign_output("ruido"), CodeSignature::Unknown);
    }

    #[test]
    fn el_entorno_declara_las_herramientas_que_faltan() {
        let report = EnvironmentReport {
            user: "test".to_owned(),
            uid: 501,
            is_root: false,
            tools: vec![
                ("/usr/sbin/lsof".to_owned(), "Conexiones".to_owned(), true),
                ("/usr/bin/inexistente".to_owned(), "Nada".to_owned(), false),
            ],
        };
        assert_eq!(report.missing_tools(), vec!["/usr/bin/inexistente"]);
    }

    #[test]
    fn el_entorno_real_encuentra_las_utilidades_base_de_macos() {
        let report = environment();
        assert!(!report.user.is_empty());
        // `launchctl` existe en cualquier macOS soportado; si faltara, el
        // diagnóstico de persistencia no tendría sentido.
        assert!(report
            .tools
            .iter()
            .any(|(path, _, available)| path.ends_with("launchctl") && *available));
    }
}
