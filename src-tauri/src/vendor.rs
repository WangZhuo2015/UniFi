//! OUI/CID vendor lookup.
//!
//! Data source:
//! - IEEE MA-L OUI registry
//! - IEEE CID registry

use std::collections::HashMap;
use std::sync::OnceLock;

const VENDOR_PREFIXES_TSV: &str = include_str!("../data/vendor-prefixes.tsv");
const PREFIX_LENGTHS: [usize; 3] = [9, 7, 6];

fn normalize_identifier(value: &str) -> Option<String> {
    let normalized: String = value
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .map(|c| c.to_ascii_uppercase())
        .collect();

    if normalized.len() < 6 {
        None
    } else {
        Some(normalized)
    }
}

fn vendor_alias(name: &'static str) -> &'static str {
    match name {
        "Extreme Networks Headquarters" => "Extreme Networks",
        "Hewlett Packard" => "HP",
        "Hewlett Packard Enterprise" => "HPE",
        "zte corporation" => "ZTE",
        "HUAWEI TECHNOLOGIES CO.,LTD" => "Huawei",
        "HUAWEI TECHNOLOGIES CO.,LTD." => "Huawei",
        "Huawei Device Co., Ltd." => "Huawei",
        "TP-LINK TECHNOLOGIES CO.,LTD." => "TP-Link",
        "TP-LINK TECHNOLOGIES CO., LTD." => "TP-Link",
        "TP-LINK TECHNOLOGIES CO.,LTD" => "TP-Link",
        "TP-Link Systems Inc." => "TP-Link",
        "ASUSTek COMPUTER INC." => "ASUS",
        "Ubiquiti Inc" => "Ubiquiti",
        "Ubiquiti Networks Inc." => "Ubiquiti",
        "Cisco Systems, Inc" => "Cisco",
        "Cisco Systems, Inc." => "Cisco",
        "Apple, Inc." => "Apple",
        "Intel Corporate" => "Intel",
        "MediaTek Inc" => "MediaTek",
        other => other,
    }
}

fn vendor_prefixes() -> &'static HashMap<&'static str, &'static str> {
    static PREFIXES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

    PREFIXES.get_or_init(|| {
        let mut map = HashMap::new();

        for line in VENDOR_PREFIXES_TSV.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let mut parts = trimmed.splitn(2, '\t');
            let Some(prefix) = parts.next() else {
                continue;
            };
            let Some(name) = parts.next() else {
                continue;
            };

            map.entry(prefix).or_insert(vendor_alias(name));
        }

        map
    })
}

fn is_locally_administered(prefix: &str) -> bool {
    u8::from_str_radix(&prefix[0..2], 16)
        .map(|first_octet| (first_octet & 0x02) != 0)
        .unwrap_or(false)
}

fn canonicalize_local_prefix(prefix: &str) -> Option<String> {
    if !is_locally_administered(prefix) {
        return None;
    }

    let first_octet = u8::from_str_radix(&prefix[0..2], 16).ok()?;
    let canonical = first_octet & !0x02;
    Some(format!("{:02X}{}", canonical, &prefix[2..]))
}

fn should_ignore_ie_vendor(vendor: &str) -> bool {
    let normalized = vendor.to_ascii_lowercase();
    normalized.contains("microsoft")
        || normalized.contains("wifi alliance")
        || normalized.contains("wi-fi alliance")
}

pub fn lookup_oui(prefix: &str) -> Option<&'static str> {
    let normalized = normalize_identifier(prefix)?;

    for length in PREFIX_LENGTHS {
        if normalized.len() < length {
            continue;
        }

        if let Some(vendor) = vendor_prefixes().get(&normalized[..length]) {
            return Some(*vendor);
        }
    }

    None
}

pub fn lookup_vendor(bssid: &str) -> String {
    if let Some(normalized) = normalize_identifier(bssid) {
        if is_locally_administered(&normalized) {
            if let Some(canonicalized) = canonicalize_local_prefix(&normalized) {
                if let Some(vendor) = lookup_oui(&canonicalized) {
                    return vendor.to_string();
                }
            }

            return "Locally Administered".to_string();
        }
    }

    lookup_oui(bssid).unwrap_or("Unknown").to_string()
}

pub fn lookup_vendor_from_ie(ie_data: &[u8]) -> Option<String> {
    let mut pos = 0usize;
    let mut best_match: Option<&'static str> = None;

    while pos + 1 < ie_data.len() {
        let id = ie_data[pos];
        let len = ie_data[pos + 1] as usize;

        if pos + 2 + len > ie_data.len() {
            break;
        }

        let data = &ie_data[pos + 2..pos + 2 + len];
        if id == 221 && data.len() >= 3 {
            let oui = format!("{:02X}{:02X}{:02X}", data[0], data[1], data[2]);
            if let Some(vendor) = lookup_oui(&oui) {
                if should_ignore_ie_vendor(vendor) {
                    pos += 2 + len;
                    continue;
                }

                if best_match.is_none() {
                    best_match = Some(vendor);
                }
            }
        }

        pos += 2 + len;
    }

    best_match.map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::{lookup_oui, lookup_vendor};

    #[test]
    fn resolves_known_official_oui_prefixes() {
        assert_eq!(lookup_oui("802D1A"), Some("ZTE"));
        assert_eq!(lookup_oui("2C704F"), Some("ZTE"));
        assert_eq!(lookup_oui("F86FB0"), Some("TP-Link"));
    }

    #[test]
    fn resolves_colon_separated_bssid() {
        assert_eq!(lookup_vendor("80:2D:1A:4B:8C:07"), "ZTE");
        assert_eq!(lookup_vendor("2C:70:4F:63:CF:DB"), "ZTE");
        assert_eq!(lookup_vendor("F8:6F:B0:A6:DE:50"), "TP-Link");
    }

    #[test]
    fn resolves_locally_administered_prefixes_via_canonical_oui() {
        assert_eq!(lookup_vendor("6A:DD:B7:78:7F:FF"), "TP-Link");
        assert_eq!(lookup_vendor("1E:3C:D4:00:AB:B0"), "Huawei");
        assert_eq!(lookup_vendor("AE:99:29:8A:B4:90"), "Huawei");
    }
}
