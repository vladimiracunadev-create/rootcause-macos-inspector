//! Equipos cercanos del segmento local (vecinos ARP/NDP).
//!
//! Responde una pregunta que ninguna vista de macOS contesta de golpe: *¿qué
//! otros equipos hay junto al mío y cuáles son conocidos?* Un dispositivo nuevo
//! en el mismo segmento puede ser el primer indicio de un intruso —un portátil
//! ajeno, un punto de acceso no autorizado, un escaneo lateral—, y un cambio en
//! la MAC de la puerta de enlace es la firma clásica de un ataque de suplantación
//! ARP.
//!
//! El escaneo **pasivo** (por defecto) solo lee la tabla de vecinos que el
//! sistema ya conoce: es instantáneo y no genera un solo paquete. El escaneo
//! **profundo** hace un barrido de descubrimiento del `/24` para despertar a los
//! equipos que aún no aparecen, y resuelve nombres. Es ruidoso y lento, así que
//! nunca se ejecuta solo.

use crate::models::{
    AnomalyEvent, IncidentEvidence, NetworkDevice, NetworkScan, PersistenceChange, RiskLevel,
    Severity, WatchedItem,
};
use crate::services::macos;
use chrono::{DateTime, Utc};

/// Prefijos OUI frecuentes en una red doméstica u oficina. No pretende ser la
/// base IEEE completa: sirve para reconocer de un vistazo si el equipo nuevo es
/// un Apple, un router o algo que no se identifica.
const OUI_TABLE: &[(&str, &str)] = &[
    ("00:03:93", "Apple"),
    ("00:1b:63", "Apple"),
    ("00:1e:c2", "Apple"),
    ("00:25:00", "Apple"),
    ("28:cf:e9", "Apple"),
    ("3c:15:c2", "Apple"),
    ("a4:83:e7", "Apple"),
    ("f0:18:98", "Apple"),
    ("00:50:56", "VMware"),
    ("00:0c:29", "VMware"),
    ("08:00:27", "VirtualBox"),
    ("52:54:00", "QEMU/KVM"),
    ("00:1a:11", "Google"),
    ("00:24:e4", "Withings"),
    ("b8:27:eb", "Raspberry Pi"),
    ("dc:a6:32", "Raspberry Pi"),
    ("e4:5f:01", "Raspberry Pi"),
    ("00:18:0a", "Cisco Meraki"),
    ("00:1d:7e", "Cisco-Linksys"),
    ("c0:56:27", "Belkin"),
    ("00:26:5a", "D-Link"),
    ("00:1f:33", "Netgear"),
    ("2c:30:33", "Netgear"),
    ("00:14:6c", "Netgear"),
    ("00:90:4c", "Epson"),
    ("00:17:88", "Philips Hue"),
];

/// Explora la red local. `deep = true` lanza primero un barrido de
/// descubrimiento y resuelve nombres por DNS inverso.
pub fn scan(deep: bool, scanned_at: &str) -> NetworkScan {
    let (adapter_name, gateway_ip) = macos::default_route();
    let (local_ip, local_mac) = macos::interface_addresses(&adapter_name);
    let subnet_prefix = subnet_prefix_of(&local_ip);

    if deep && !subnet_prefix.is_empty() {
        macos::discovery_sweep(&subnet_prefix);
    }

    let raw = macos::arp_table().unwrap_or_default();
    let mut devices = parse_arp_table(&raw);

    for device in devices.iter_mut() {
        device.is_gateway = !gateway_ip.is_empty() && device.ip == gateway_ip;
        device.is_self = !local_ip.is_empty() && device.ip == local_ip;
        if deep {
            device.hostname = macos::reverse_dns(&device.ip);
        }
        classify_device(device);
    }

    // El propio equipo casi nunca aparece en su tabla ARP; se añade para que la
    // vista sea completa y para no marcarlo como "desaparecido" en la baseline.
    if !local_ip.is_empty() && !devices.iter().any(|device| device.ip == local_ip) {
        let mut own = NetworkDevice {
            ip: local_ip.clone(),
            mac: normalize_mac(&local_mac),
            hostname: "este equipo".to_owned(),
            interface: adapter_name.clone(),
            state: "local".to_owned(),
            is_self: true,
            ..Default::default()
        };
        own.vendor = vendor_from_mac(&own.mac);
        classify_device(&mut own);
        devices.push(own);
    }

    devices.sort_by(|left, right| {
        right
            .severity
            .cmp(&left.severity)
            .then_with(|| ip_sort_key(&left.ip).cmp(&ip_sort_key(&right.ip)))
    });

    let total_devices = devices.iter().filter(|device| !device.is_self).count();

    NetworkScan {
        scanned_at: scanned_at.to_owned(),
        adapter_name,
        local_ip,
        local_mac: normalize_mac(&local_mac),
        subnet_prefix,
        gateway_ip,
        deep,
        devices,
        total_devices,
        new_devices: 0,
        limitations: vec![
            "El escaneo pasivo solo ve equipos con los que este Mac ya habló; usa el escaneo profundo para descubrir el resto."
                .to_owned(),
            "Una MAC puede falsificarse: la baseline detecta cambios, no garantiza identidad."
                .to_owned(),
            "No sustituye a un IDS ni a un análisis forense de red completo.".to_owned(),
        ],
    }
}

/// Convierte la salida de `arp -a -n` en dispositivos.
///
/// Formato: `? (192.168.1.1) at 0:11:22:33:44:55 on en0 ifscope [ethernet]`.
/// Se descartan las entradas incompletas (`(incomplete)`) y las de difusión.
pub fn parse_arp_table(raw: &str) -> Vec<NetworkDevice> {
    let mut devices = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some(ip) = line
            .split_once('(')
            .and_then(|(_, rest)| rest.split_once(')'))
            .map(|(ip, _)| ip.trim().to_owned())
        else {
            continue;
        };

        let Some(after_at) = line.split(" at ").nth(1) else {
            continue;
        };
        let mac_raw = after_at.split_whitespace().next().unwrap_or_default();
        if mac_raw.eq_ignore_ascii_case("(incomplete)") || !mac_raw.contains(':') {
            continue;
        }
        let mac = normalize_mac(mac_raw);
        if mac == "ff:ff:ff:ff:ff:ff" {
            continue;
        }

        let interface = line
            .split(" on ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .unwrap_or_default()
            .to_owned();

        let state = if line.contains("permanent") {
            "permanent".to_owned()
        } else {
            "reachable".to_owned()
        };

        devices.push(NetworkDevice {
            vendor: vendor_from_mac(&mac),
            ip,
            mac,
            interface,
            state,
            ..Default::default()
        });
    }

    devices
}

/// Normaliza una MAC a `aa:bb:cc:dd:ee:ff`: macOS imprime los octetos sin cero
/// a la izquierda (`0:11:22`), lo que rompería la comparación con la baseline.
pub fn normalize_mac(mac: &str) -> String {
    if mac.trim().is_empty() {
        return String::new();
    }
    mac.split(':')
        .map(|octet| format!("{:0>2}", octet.trim().to_ascii_lowercase()))
        .collect::<Vec<_>>()
        .join(":")
}

/// Fabricante aproximado a partir del prefijo OUI.
pub fn vendor_from_mac(mac: &str) -> String {
    let normalized = normalize_mac(mac);
    if normalized.len() < 8 {
        return String::new();
    }
    let prefix = &normalized[..8];
    OUI_TABLE
        .iter()
        .find(|(oui, _)| *oui == prefix)
        .map(|(_, vendor)| (*vendor).to_owned())
        .unwrap_or_default()
}

/// Prefijo `/24` de una IPv4 (`192.168.1.7` → `192.168.1`).
pub fn subnet_prefix_of(ip: &str) -> String {
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() == 4 {
        format!("{}.{}.{}", parts[0], parts[1], parts[2])
    } else {
        String::new()
    }
}

/// Clave estable de un dispositivo: la MAC, o la IP si no hay MAC.
pub fn device_key(device: &NetworkDevice) -> String {
    if device.mac.is_empty() {
        format!("ip:{}", device.ip)
    } else {
        format!("mac:{}", device.mac)
    }
}

/// Asigna severidad y explicación según lo conocido que sea el dispositivo.
///
/// El caso crítico es el cambio de MAC de la puerta de enlace: si el router
/// "cambió de tarjeta de red" entre dos capturas, lo más probable es que alguien
/// esté suplantándolo.
pub fn classify_device(device: &mut NetworkDevice) {
    if device.is_self {
        device.severity = Severity::Healthy;
        device.reason = "Este equipo".to_owned();
        return;
    }

    match device.change_status {
        PersistenceChange::Added if device.is_gateway => {
            device.severity = Severity::Critical;
            device.reason =
                "La puerta de enlace cambió de MAC: posible suplantación ARP".to_owned();
        }
        PersistenceChange::Added => {
            device.severity = Severity::Warning;
            device.reason = "Equipo nuevo respecto a la red conocida".to_owned();
        }
        PersistenceChange::Modified => {
            device.severity = Severity::Warning;
            device.reason = "El dispositivo cambió respecto a la baseline".to_owned();
        }
        PersistenceChange::Removed => {
            device.severity = Severity::Healthy;
            device.reason = "Estaba en la red conocida y ya no responde".to_owned();
        }
        PersistenceChange::Unchanged => {
            device.severity = Severity::Healthy;
            device.reason = if device.is_gateway {
                "Puerta de enlace conocida".to_owned()
            } else {
                "Equipo conocido de la red".to_owned()
            };
        }
    }
}

/// Convierte los dispositivos en ítems vigilables por el motor de baseline.
pub fn device_watch_items(devices: &[NetworkDevice]) -> Vec<WatchedItem> {
    devices
        .iter()
        .filter(|device| !device.is_self)
        .map(|device| WatchedItem {
            key: device_key(device),
            // El valor vigilado es la IP + si es puerta de enlace: así un cambio
            // de rol (una MAC nueva haciendo de router) se detecta como cambio.
            value: format!("{}|{}", device.ip, device.is_gateway),
            label: if device.hostname.is_empty() {
                device.ip.clone()
            } else {
                format!("{} ({})", device.hostname, device.ip)
            },
            detail: format!("MAC {} · {}", device.mac, device.interface),
            change_status: Default::default(),
        })
        .collect()
}

/// Reconstruye un dispositivo "desaparecido" a partir de su ítem de baseline,
/// para que la vista muestre lo que ya no responde.
pub fn device_from_watch_item(item: &WatchedItem) -> NetworkDevice {
    let ip = item.value.split('|').next().unwrap_or_default().to_owned();
    let mac = item.key.strip_prefix("mac:").unwrap_or_default().to_owned();
    let mut device = NetworkDevice {
        vendor: vendor_from_mac(&mac),
        ip,
        mac,
        state: "ausente".to_owned(),
        change_status: PersistenceChange::Removed,
        ..Default::default()
    };
    classify_device(&mut device);
    device
}

/// Evento de anomalía por dispositivo nuevo o cambiado en la red conocida.
pub fn new_device_event(
    detected_at: DateTime<Utc>,
    device: &NetworkDevice,
) -> Option<AnomalyEvent> {
    if !matches!(
        device.change_status,
        PersistenceChange::Added | PersistenceChange::Modified
    ) {
        return None;
    }

    let (severity, score, title) = if device.is_gateway {
        (
            RiskLevel::Critical,
            92_u16,
            "La puerta de enlace cambió de MAC",
        )
    } else {
        (RiskLevel::Medium, 48, "Equipo desconocido en la red local")
    };

    Some(AnomalyEvent {
        event_id: format!(
            "anom-{}-unknown-device-{}",
            detected_at.timestamp_millis(),
            device_key(device)
        ),
        detected_at,
        severity,
        score,
        status: "open".to_owned(),
        kind: "unknown-device".to_owned(),
        title: title.to_owned(),
        summary: format!(
            "El dispositivo {} ({}) no estaba en la baseline de red conocida.",
            device.ip,
            if device.mac.is_empty() {
                "sin MAC"
            } else {
                &device.mac
            }
        ),
        root_cause_hypothesis: if device.is_gateway {
            "posible suplantación ARP de la puerta de enlace".to_owned()
        } else {
            "un equipo nuevo se unió al segmento de red".to_owned()
        },
        recommended_action: if device.is_gateway {
            "Verifica físicamente el router antes de seguir usando la red.".to_owned()
        } else {
            "Confirma si el equipo es tuyo y acepta la baseline de red si es legítimo.".to_owned()
        },
        evidence: vec![
            IncidentEvidence {
                kind: "ip".to_owned(),
                label: "IP".to_owned(),
                value: device.ip.clone(),
            },
            IncidentEvidence {
                kind: "mac".to_owned(),
                label: "MAC".to_owned(),
                value: device.mac.clone(),
            },
            IncidentEvidence {
                kind: "vendor".to_owned(),
                label: "Fabricante".to_owned(),
                value: if device.vendor.is_empty() {
                    "no identificado".to_owned()
                } else {
                    device.vendor.clone()
                },
            },
        ],
        ..Default::default()
    })
}

/// Clave de orden numérica para IPv4, para que `.10` no quede antes que `.2`.
fn ip_sort_key(ip: &str) -> (u8, u8, u8, u8) {
    let mut octets = [0_u8; 4];
    for (index, part) in ip.split('.').take(4).enumerate() {
        octets[index] = part.parse().unwrap_or(0);
    }
    (octets[0], octets[1], octets[2], octets[3])
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARP_SAMPLE: &str = "? (192.168.1.1) at 0:11:22:33:44:55 on en0 ifscope [ethernet]\n\
? (192.168.1.20) at a4:83:e7:1:2:3 on en0 ifscope [ethernet]\n\
? (192.168.1.255) at ff:ff:ff:ff:ff:ff on en0 ifscope [ethernet]\n\
? (192.168.1.44) at (incomplete) on en0 ifscope [ethernet]\n";

    #[test]
    fn parsea_vecinos_y_descarta_ruido() {
        let devices = parse_arp_table(ARP_SAMPLE);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].ip, "192.168.1.1");
        assert_eq!(devices[0].interface, "en0");
    }

    #[test]
    fn normaliza_octetos_sin_cero_a_la_izquierda() {
        assert_eq!(normalize_mac("0:11:22:33:44:55"), "00:11:22:33:44:55");
        assert_eq!(normalize_mac("A4:83:E7:1:2:3"), "a4:83:e7:01:02:03");
        assert!(normalize_mac("").is_empty());
    }

    #[test]
    fn reconoce_fabricante_por_oui() {
        assert_eq!(vendor_from_mac("a4:83:e7:1:2:3"), "Apple");
        assert_eq!(vendor_from_mac("b8:27:eb:0:0:1"), "Raspberry Pi");
        assert!(vendor_from_mac("de:ad:be:ef:00:01").is_empty());
    }

    #[test]
    fn prefijo_de_subred_solo_para_ipv4() {
        assert_eq!(subnet_prefix_of("192.168.1.7"), "192.168.1");
        assert!(subnet_prefix_of("fe80::1").is_empty());
    }

    #[test]
    fn gateway_nuevo_es_incidente_critico() {
        let device = NetworkDevice {
            ip: "192.168.1.1".to_owned(),
            mac: "00:11:22:33:44:55".to_owned(),
            is_gateway: true,
            change_status: PersistenceChange::Added,
            ..Default::default()
        };
        let event = new_device_event(Utc::now(), &device).expect("debe generar evento");
        assert_eq!(event.severity, RiskLevel::Critical);
        assert_eq!(event.kind, "unknown-device");
    }

    #[test]
    fn dispositivo_sin_cambios_no_genera_evento() {
        let device = NetworkDevice {
            ip: "192.168.1.30".to_owned(),
            ..Default::default()
        };
        assert!(new_device_event(Utc::now(), &device).is_none());
    }

    #[test]
    fn el_propio_equipo_no_entra_en_la_baseline() {
        let devices = vec![
            NetworkDevice {
                ip: "192.168.1.5".to_owned(),
                mac: "00:11:22:33:44:55".to_owned(),
                is_self: true,
                ..Default::default()
            },
            NetworkDevice {
                ip: "192.168.1.1".to_owned(),
                mac: "aa:bb:cc:dd:ee:ff".to_owned(),
                ..Default::default()
            },
        ];
        let items = device_watch_items(&devices);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].key, "mac:aa:bb:cc:dd:ee:ff");
    }

    #[test]
    fn ordena_ips_numericamente() {
        assert!(ip_sort_key("192.168.1.2") < ip_sort_key("192.168.1.10"));
    }
}
