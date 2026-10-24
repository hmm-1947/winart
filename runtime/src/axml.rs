use std::collections::HashMap;

pub struct Manifest {
    pub package: String,
    pub launcher: Option<String>,
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,
    pub permissions: Vec<String>,
}

fn u16le(d: &[u8], o: usize) -> usize {
    u16::from_le_bytes([d[o], d[o + 1]]) as usize
}

fn u32le(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

fn parse_pool(d: &[u8], base: usize) -> Vec<String> {
    let count = u32le(d, base + 8) as usize;
    let flags = u32le(d, base + 16);
    let starts = u32le(d, base + 20) as usize;
    let utf8 = flags & 0x100 != 0;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let off = u32le(d, base + 28 + i * 4) as usize;
        let mut p = base + starts + off;
        if utf8 {
            let mut l = d[p] as usize;
            p += 1;
            if l & 0x80 != 0 {
                l = ((l & 0x7f) << 8) | d[p] as usize;
                p += 1;
            }
            let mut bl = d[p] as usize;
            p += 1;
            if bl & 0x80 != 0 {
                bl = ((bl & 0x7f) << 8) | d[p] as usize;
                p += 1;
            }
            let _ = l;
            out.push(String::from_utf8_lossy(&d[p..p + bl]).into_owned());
        } else {
            let mut l = u16le(d, p);
            p += 2;
            if l & 0x8000 != 0 {
                l = ((l & 0x7fff) << 16) | u16le(d, p);
                p += 2;
            }
            let units: Vec<u16> = (0..l).map(|k| u16le(d, p + k * 2) as u16).collect();
            out.push(String::from_utf16_lossy(&units));
        }
    }
    out
}

pub fn parse_manifest(d: &[u8]) -> Result<Manifest, String> {
    if d.len() < 8 || u16le(d, 0) != 0x0003 {
        return Err("not binary xml".into());
    }
    let mut pool: Vec<String> = Vec::new();
    let mut p = u16le(d, 2);
    let mut m = Manifest { package: String::new(), launcher: None, min_sdk: None, target_sdk: None, permissions: vec![] };
    let mut cur_activity = String::new();
    let mut has_main = false;
    let mut has_launcher = false;
    while p + 8 <= d.len() {
        let ty = u16le(d, p);
        let size = u32le(d, p + 4) as usize;
        if size == 0 {
            break;
        }
        match ty {
            0x0001 => pool = parse_pool(d, p),
            0x0102 => {
                let name = pool.get(u32le(d, p + 20) as usize).cloned().unwrap_or_default();
                let astart = u16le(d, p + 24);
                let acount = u16le(d, p + 28);
                let mut attrs: HashMap<String, (String, u32)> = HashMap::new();
                for k in 0..acount {
                    let a = p + 16 + astart + k * 20;
                    let an = pool.get(u32le(d, a + 4) as usize).cloned().unwrap_or_default();
                    let raw = u32le(d, a + 8);
                    let dt = d[a + 15];
                    let data = u32le(d, a + 16);
                    let sv = if raw != 0xffffffff {
                        pool.get(raw as usize).cloned().unwrap_or_default()
                    } else if dt == 3 {
                        pool.get(data as usize).cloned().unwrap_or_default()
                    } else {
                        data.to_string()
                    };
                    attrs.insert(an, (sv, data));
                }
                match name.as_str() {
                    "manifest" => m.package = attrs.get("package").map(|v| v.0.clone()).unwrap_or_default(),
                    "uses-sdk" => {
                        m.min_sdk = attrs.get("minSdkVersion").map(|v| v.1);
                        m.target_sdk = attrs.get("targetSdkVersion").map(|v| v.1);
                    }
                    "uses-permission" => {
                        if let Some(v) = attrs.get("name") {
                            m.permissions.push(v.0.clone());
                        }
                    }
                    "activity" | "activity-alias" => {
                        cur_activity = attrs.get("name").map(|v| v.0.clone()).unwrap_or_default();
                        has_main = false;
                        has_launcher = false;
                    }
                    "action" => {
                        if attrs.get("name").map(|v| v.0.as_str()) == Some("android.intent.action.MAIN") {
                            has_main = true;
                        }
                    }
                    "category" => {
                        if attrs.get("name").map(|v| v.0.as_str()) == Some("android.intent.category.LAUNCHER") {
                            has_launcher = true;
                        }
                    }
                    _ => {}
                }
            }
            0x0103 => {
                let name = pool.get(u32le(d, p + 20) as usize).cloned().unwrap_or_default();
                if (name == "activity" || name == "activity-alias") && has_main && has_launcher && m.launcher.is_none() {
                    let full = if cur_activity.starts_with('.') {
                        format!("{}{}", m.package, cur_activity)
                    } else if !cur_activity.contains('.') {
                        format!("{}.{}", m.package, cur_activity)
                    } else {
                        cur_activity.clone()
                    };
                    m.launcher = Some(full);
                }
            }
            _ => {}
        }
        p += size;
    }
    Ok(m)
}
