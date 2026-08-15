//! Conexiones activas por proceso, a partir de `lsof -i` en modo campo.
//!
//! La pregunta que responde este módulo es concreta: *¿qué binario mantiene
//! conversaciones con el exterior, y hacia dónde?* Un proceso legítimo hablando
//! con una IP pública es lo normal; un binario sin firmar en `/tmp` hablando con
//! cuatro destinos públicos distintos ya no lo es.

use crate::models::{ConnectionInsight, Severity};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Registro intermedio mientras se acumulan los campos de `lsof -F`.
#[derive(Default, Clone)]
struct FieldState {
    pid: u32,
    command: String,
    user: String,
    protocol: String,
    family: String,
    name: String,
    state: String,
}

/// Convierte la salida por campos de `lsof -i -FpcLftPnT` en conexiones.
///
/// El formato es una secuencia de líneas `<letra><valor>`: `p` abre un proceso,
/// `f` abre un descriptor dentro de ese proceso, y el resto son atributos del
/// descriptor abierto. Se emite un registro cada vez que empieza uno nuevo y al
/// terminar la entrada.
pub fn parse_lsof_field_output(
    output: &str,
    process_paths: &HashMap<u32, String>,
) -> Vec<ConnectionInsight> {
    let mut connections = Vec::new();
    let mut current = FieldState::default();
    let mut has_descriptor = false;

    for line in output.lines() {
        let Some(tag) = line.chars().next() else {
            continue;
        };
        let value = &line[tag.len_utf8()..];

        match tag {
            'p' => {
                if has_descriptor {
                    push_connection(&mut connections, &current, process_paths);
                    has_descriptor = false;
                }
                current.pid = value.trim().parse().unwrap_or(0);
                current.command.clear();
                current.user.clear();
            }
            'c' => current.command = value.trim().to_owned(),
            'L' => current.user = value.trim().to_owned(),
            'f' => {
                if has_descriptor {
                    push_connection(&mut connections, &current, process_paths);
                }
                has_descriptor = true;
                current.protocol.clear();
                current.family.clear();
                current.name.clear();
                current.state.clear();
            }
            't' => current.family = value.trim().to_owned(),
            'P' => current.protocol = value.trim().to_owned(),
            'n' => current.name = value.trim().to_owned(),
            'T' => {
                // Los campos TCP llegan como `TST=ESTABLISHED`, `TQR=0`…
                if let Some(state) = value.strip_prefix("ST=") {
                    current.state = state.trim().to_owned();
                }
            }
            _ => {}
        }
    }

    if has_descriptor {
        push_connection(&mut connections, &current, process_paths);
    }

    connections.sort_by(|left, right| {
        right
            .severity
            .cmp(&left.severity)
            .then_with(|| left.process_name.cmp(&right.process_name))
    });
    connections
}

fn push_connection(
    connections: &mut Vec<ConnectionInsight>,
    state: &FieldState,
    process_paths: &HashMap<u32, String>,
) {
    if state.name.is_empty() {
        return;
    }

    let (local_address, remote_address) = split_endpoints(&state.name);
    let is_listening = state.state.eq_ignore_ascii_case("LISTEN")
        || (remote_address.is_empty() && state.name.contains('*'));
    let is_public_remote = extract_ip(&remote_address)
        .map(|ip| is_public_ip(&ip))
        .unwrap_or(false);

    let (severity, reason) = classify_connection(
        is_listening,
        is_public_remote,
        &local_address,
        &remote_address,
    );

    connections.push(ConnectionInsight {
        protocol: if state.protocol.is_empty() {
            state.family.clone()
        } else {
            state.protocol.clone()
        },
        local_address,
        remote_address,
        state: state.state.clone(),
        pid: state.pid,
        process_name: state.command.clone(),
        exe_path: process_paths.get(&state.pid).cloned().unwrap_or_default(),
        user: state.user.clone(),
        severity,
        reason,
        is_public_remote,
        is_listening,
    });
}

/// Separa `local->remoto` en sus dos extremos. Un socket a la escucha solo trae
/// el extremo local.
fn split_endpoints(name: &str) -> (String, String) {
    match name.split_once("->") {
        Some((local, remote)) => (local.trim().to_owned(), remote.trim().to_owned()),
        None => (name.trim().to_owned(), String::new()),
    }
}

/// Asigna severidad y explicación a una conexión.
///
/// Escuchar en `0.0.0.0`/`*` es la señal más accionable de todas: significa que
/// el puerto está expuesto a toda la red, no solo al propio equipo.
pub fn classify_connection(
    is_listening: bool,
    is_public_remote: bool,
    local_address: &str,
    remote_address: &str,
) -> (Severity, String) {
    if is_listening {
        let exposed = local_address.starts_with("*:")
            || local_address.starts_with("0.0.0.0:")
            || local_address.starts_with("[::]:");
        return if exposed {
            (
                Severity::Warning,
                "Puerto a la escucha expuesto a toda la red".to_owned(),
            )
        } else {
            (
                Severity::Healthy,
                "Puerto a la escucha solo en la interfaz local".to_owned(),
            )
        };
    }

    if is_public_remote {
        return (
            Severity::Warning,
            format!("Conexión saliente a IP pública ({remote_address})"),
        );
    }

    if remote_address.is_empty() {
        (Severity::Healthy, "Socket sin extremo remoto".to_owned())
    } else {
        (
            Severity::Healthy,
            "Conexión dentro de la red local o al propio equipo".to_owned(),
        )
    }
}

/// Extrae la IP de un extremo `host:puerto`, tolerando IPv6 entre corchetes.
pub fn extract_ip(endpoint: &str) -> Option<String> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return None;
    }

    if let Some(rest) = endpoint.strip_prefix('[') {
        return rest.split_once(']').map(|(ip, _)| ip.to_owned());
    }

    // `lsof` puede devolver un nombre de host si no se usó `-n`; se acepta igual
    // y quien clasifique decidirá que no es una IP pública reconocible.
    match endpoint.rsplit_once(':') {
        Some((host, _)) if !host.is_empty() => Some(host.to_owned()),
        _ => Some(endpoint.to_owned()),
    }
}

/// `true` si la IP es enrutable en Internet (no privada, ni loopback, ni
/// link-local, ni multicast).
pub fn is_public_ip(value: &str) -> bool {
    let Ok(ip) = value.parse::<IpAddr>() else {
        return false;
    };
    match ip {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => is_public_v6(v6),
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    !(ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_multicast()
        || ip.is_unspecified()
        || ip.is_documentation()
        // Carrier-grade NAT (100.64.0.0/10): tampoco es Internet abierta.
        || (ip.octets()[0] == 100 && (64..128).contains(&ip.octets()[1])))
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    if ip.is_loopback() || ip.is_multicast() || ip.is_unspecified() {
        return false;
    }
    let segments = ip.segments();
    // fc00::/7 (unique local) y fe80::/10 (link-local) no son públicas.
    let unique_local = (segments[0] & 0xfe00) == 0xfc00;
    let link_local = (segments[0] & 0xffc0) == 0xfe80;
    !(unique_local || link_local)
}

/// Cuenta destinos públicos distintos por PID. Es el insumo de la heurística de
/// "muchos destinos públicos", que separa un navegador (decenas de destinos,
/// firmado, en `/Applications`) de un binario suelto que abre cuatro sesiones.
pub fn unique_public_remotes_by_pid(
    connections: &[ConnectionInsight],
) -> HashMap<u32, Vec<String>> {
    let mut table: HashMap<u32, Vec<String>> = HashMap::new();
    for connection in connections.iter().filter(|item| item.is_public_remote) {
        let Some(ip) = extract_ip(&connection.remote_address) else {
            continue;
        };
        let entry = table.entry(connection.pid).or_default();
        if !entry.contains(&ip) {
            entry.push(ip);
        }
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "p501\ncGoogle Chrome\nLvladimir\nf35\ntIPv4\nPTCP\nn192.168.1.5:54321->142.250.1.1:443\nTST=ESTABLISHED\nf40\ntIPv4\nPTCP\nn*:8080\nTST=LISTEN\np777\ncsshd\nLroot\nf3\ntIPv4\nPTCP\nn127.0.0.1:22\nTST=LISTEN\n";

    #[test]
    fn parsea_nombres_de_proceso_con_espacios() {
        let connections = parse_lsof_field_output(SAMPLE, &HashMap::new());
        assert_eq!(connections.len(), 3);
        assert!(connections
            .iter()
            .any(|item| item.process_name == "Google Chrome"));
    }

    #[test]
    fn detecta_destino_publico_y_puerto_expuesto() {
        let connections = parse_lsof_field_output(SAMPLE, &HashMap::new());

        let outbound = connections
            .iter()
            .find(|item| item.remote_address == "142.250.1.1:443")
            .expect("conexión saliente");
        assert!(outbound.is_public_remote);
        assert_eq!(outbound.state, "ESTABLISHED");

        let exposed = connections
            .iter()
            .find(|item| item.local_address == "*:8080")
            .expect("puerto expuesto");
        assert!(exposed.is_listening);
        assert_eq!(exposed.severity, Severity::Warning);

        let local_only = connections
            .iter()
            .find(|item| item.local_address == "127.0.0.1:22")
            .expect("puerto local");
        assert_eq!(local_only.severity, Severity::Healthy);
    }

    #[test]
    fn ips_privadas_y_loopback_no_son_publicas() {
        assert!(!is_public_ip("192.168.1.10"));
        assert!(!is_public_ip("10.0.0.1"));
        assert!(!is_public_ip("172.16.5.4"));
        assert!(!is_public_ip("127.0.0.1"));
        assert!(!is_public_ip("169.254.1.1"));
        assert!(!is_public_ip("100.64.0.1"));
        assert!(!is_public_ip("::1"));
        assert!(!is_public_ip("fe80::1"));
        assert!(!is_public_ip("no-es-una-ip"));
    }

    #[test]
    fn ips_publicas_se_reconocen() {
        assert!(is_public_ip("142.250.1.1"));
        assert!(is_public_ip("8.8.8.8"));
        assert!(is_public_ip("2606:4700::1111"));
    }

    #[test]
    fn extrae_ip_de_extremos_v4_y_v6() {
        assert_eq!(
            extract_ip("192.168.1.5:443"),
            Some("192.168.1.5".to_owned())
        );
        assert_eq!(
            extract_ip("[2606:4700::1111]:443"),
            Some("2606:4700::1111".to_owned())
        );
        assert_eq!(extract_ip(""), None);
    }

    #[test]
    fn agrupa_destinos_publicos_unicos_por_pid() {
        let connections = vec![
            ConnectionInsight {
                pid: 10,
                remote_address: "8.8.8.8:443".to_owned(),
                is_public_remote: true,
                ..Default::default()
            },
            ConnectionInsight {
                pid: 10,
                remote_address: "8.8.8.8:80".to_owned(),
                is_public_remote: true,
                ..Default::default()
            },
            ConnectionInsight {
                pid: 10,
                remote_address: "1.1.1.1:443".to_owned(),
                is_public_remote: true,
                ..Default::default()
            },
        ];
        let table = unique_public_remotes_by_pid(&connections);
        assert_eq!(table[&10].len(), 2);
    }
}
