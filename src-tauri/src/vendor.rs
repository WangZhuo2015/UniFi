//! OUI Vendor Lookup
//!
//! Simple prefix matching for common vendors.

pub fn lookup_vendor(bssid: &str) -> String {
    let oui = bssid.replace(":", "").replace("-", "").to_uppercase();
    if oui.len() < 6 {
        return "Unknown".into();
    }
    
    let prefix = &oui[0..6];
    
    const VENDORS: &[(&str, &str)] = &[
        // Apple
        ("001A2B", "TP-Link"), ("001E58", "ASUSTek"), ("00226B", "Cisco"),
        ("00246C", "Apple"), ("005056", "VMware"), ("04D4C4", "Apple"),
        ("086698", "Apple"), ("0C4DE9", "Apple"), ("10E341", "Huawei"),
        ("18A6F7", "Xiaomi"), ("2034FB", "Apple"), ("240A64", "Xiaomi"),
        ("30074D", "Apple"), ("3423BA", "Apple"), ("38F9D3", "Apple"),
        ("44D884", "Apple"), ("483B38", "Apple"), ("5C5948", "Intel"),
        ("68DBCA", "Apple"), ("6C5C14", "TP-Link"), ("784F43", "Apple"),
        ("7C6D62", "Apple"), ("7CD1C3", "Intel"), ("80EAD2", "Ubiquiti"),
        ("849FAD", "Apple"), ("90B0ED", "Xiaomi"), ("94BF2D", "Cisco"),
        ("94F6A3", "Apple"), ("9C2AA4", "Ubiquiti"), ("9CF48E", "Apple"),
        ("A01828", "Ubiquiti"), ("A4B197", "TP-Link"), ("ACF7F3", "Apple"),
        ("B06EBF", "Ubiquiti"), ("B827EB", "Raspberry Pi"), ("BC52B7", "Apple"),
        ("C069CD", "Apple"), ("D461DA", "Apple"), ("D81C79", "Apple"),
        ("F4D884", "Apple"), ("68DDB7", "Xiaomi"), ("089AC7", "Xiaomi"),
        ("F0B429", "Samsung"), ("001632", "Samsung"), ("002312", "Samsung"),
        ("001CB3", "Dell"), ("001E4D", "Dell"), ("0020ED", "Dell"),
        ("0013E8", "Nintendo"), ("00265B", "NVIDIA"), ("0024BE", "NVIDIA"),
        ("0018F8", "Netgear"), ("0024B2", "Netgear"), ("0C47C9", "Netgear"),
        ("001A70", "Linksys"), ("0022B0", "Linksys"), ("48F8B3", "Linksys"),
        ("001EE6", "D-Link"), ("002401", "D-Link"), ("00195B", "D-Link"),
    ];
    
    for (p, v) in VENDORS {
        if prefix == *p {
            return (*v).into();
        }
    }
    
    "Unknown".into()
}
