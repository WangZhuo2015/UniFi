//! OUI Vendor Lookup
//!
//! Curated prefix matching for common Wi-Fi vendors and chipsets.

fn normalize_prefix(value: &str) -> Option<String> {
    let normalized: String = value
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .map(|c| c.to_ascii_uppercase())
        .collect();

    if normalized.len() < 6 {
        None
    } else {
        Some(normalized[..6].to_string())
    }
}

pub fn lookup_oui(prefix: &str) -> Option<&'static str> {
    let normalized = normalize_prefix(prefix)?;

    const VENDORS: &[(&str, &str)] = &[
        ("00037F", "Atheros"),
        ("000CE5", "Apple"),
        ("000F66", "Cisco"),
        ("001018", "Broadcom"),
        ("001346", "Cisco"),
        ("0017F2", "Apple"),
        ("001A11", "Google"),
        ("001A2B", "TP-Link"),
        ("001A70", "Linksys"),
        ("001CB3", "Dell"),
        ("001D7E", "Apple"),
        ("001E58", "ASUS"),
        ("001EE6", "D-Link"),
        ("002147", "Netgear"),
        ("00226B", "Cisco"),
        ("00226C", "Linksys"),
        ("002401", "D-Link"),
        ("0024A5", "Ubiquiti"),
        ("0024BE", "NVIDIA"),
        ("00265B", "NVIDIA"),
        ("0026F2", "Netgear"),
        ("005056", "VMware"),
        ("0050F2", "Microsoft"),
        ("00904C", "Broadcom"),
        ("00A0C6", "Qualcomm"),
        ("04D4C4", "Apple"),
        ("086698", "Apple"),
        ("089AC7", "Xiaomi"),
        ("0C47C9", "Netgear"),
        ("0C4DE9", "Apple"),
        ("0CB694", "Huawei"),
        ("10E341", "Huawei"),
        ("14CC20", "H3C"),
        ("18A6F7", "Xiaomi"),
        ("1C5F2B", "ASUS"),
        ("2034FB", "Apple"),
        ("240A64", "Xiaomi"),
        ("2C3A28", "Aruba"),
        ("2C54CF", "Ubiquiti"),
        ("30074D", "Apple"),
        ("3423BA", "Apple"),
        ("38F9D3", "Apple"),
        ("3C84A0", "Huawei"),
        ("44D884", "Apple"),
        ("483B38", "Intel"),
        ("48F8B3", "Linksys"),
        ("4C5E0C", "Samsung"),
        ("506F9A", "Qualcomm"),
        ("50C7BF", "TP-Link"),
        ("542696", "Xiaomi"),
        ("5C5948", "Intel"),
        ("603197", "ZTE"),
        ("68DDB7", "Xiaomi"),
        ("6C5C14", "TP-Link"),
        ("7071BC", "Google"),
        ("784F43", "Apple"),
        ("7C6D62", "Apple"),
        ("7CD1C3", "Intel"),
        ("80EAD2", "Ubiquiti"),
        ("849FAD", "Apple"),
        ("8C53C3", "Aruba"),
        ("8CFDF0", "Qualcomm"),
        ("90B0ED", "Xiaomi"),
        ("94BF2D", "Cisco"),
        ("94F6A3", "Apple"),
        ("988B5D", "TP-Link"),
        ("9C2AA4", "Ubiquiti"),
        ("9CF48E", "Apple"),
        ("A01828", "Ubiquiti"),
        ("A42BB0", "TP-Link"),
        ("A4B197", "TP-Link"),
        ("A8DA0C", "Huawei"),
        ("AC84C6", "TP-Link"),
        ("ACF7F3", "Apple"),
        ("B06EBF", "Ubiquiti"),
        ("B827EB", "Raspberry Pi"),
        ("BC52B7", "Apple"),
        ("C069CD", "Apple"),
        ("C83A35", "Tenda"),
        ("CC2D21", "Netgear"),
        ("D03745", "TP-Link"),
        ("D461DA", "Apple"),
        ("D81C79", "Apple"),
        ("DCFE18", "Huawei"),
        ("E4956E", "Ubiquiti"),
        ("E4F4C6", "Netgear"),
        ("E848B8", "TP-Link"),
        ("EC172F", "H3C"),
        ("F0B429", "Samsung"),
        ("F4D884", "Apple"),
        ("F8A45F", "Xiaomi"),
        ("FCECDA", "Cisco"),
    ];

    VENDORS
        .iter()
        .find(|(known_prefix, _)| *known_prefix == normalized)
        .map(|(_, vendor)| *vendor)
}

pub fn lookup_vendor(bssid: &str) -> String {
    if let Some(normalized) = normalize_prefix(bssid) {
        if let Ok(first_octet) = u8::from_str_radix(&normalized[0..2], 16) {
            if (first_octet & 0x02) != 0 {
                return "Locally Administered".to_string();
            }
        }
    }

    lookup_oui(bssid).unwrap_or("Unknown").to_string()
}
