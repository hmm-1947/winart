pub struct Manifest {
    pub package: String,
    pub launcher: Option<String>,
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,
    pub permissions: Vec<String>,
    pub label: Option<String>,
    pub label_res: Option<u32>,
}

pub struct XmlAttr {
    pub ns: String,
    pub name: String,
    pub res_id: u32,
    pub raw: Option<String>,
    pub ty: u8,
    pub data: u32,
}

pub enum XmlEv {
    Start(String, Vec<XmlAttr>),
    End(String),
}

pub fn u16le(d: &[u8], o: usize) -> usize {
    u16::from_le_bytes([d[o], d[o + 1]]) as usize
}

pub fn u32le(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

pub fn parse_pool(d: &[u8], base: usize) -> Vec<String> {
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

pub fn parse_xml(d: &[u8]) -> Result<Vec<XmlEv>, String> {
    if d.len() < 8 || u16le(d, 0) != 0x0003 {
        return Err("not binary xml".into());
    }
    let mut pool: Vec<String> = Vec::new();
    let mut resmap: Vec<u32> = Vec::new();
    let mut out = Vec::new();
    let mut p = u16le(d, 2);
    let s = |pool: &Vec<String>, i: u32| -> String { pool.get(i as usize).cloned().unwrap_or_default() };
    while p + 8 <= d.len() {
        let ty = u16le(d, p);
        let size = u32le(d, p + 4) as usize;
        if size == 0 {
            break;
        }
        match ty {
            0x0001 => pool = parse_pool(d, p),
            0x0180 => {
                resmap = (0..(size - 8) / 4).map(|k| u32le(d, p + 8 + k * 4)).collect();
            }
            0x0102 => {
                let name = s(&pool, u32le(d, p + 20));
                let astart = u16le(d, p + 24);
                let asize = u16le(d, p + 26);
                let acount = u16le(d, p + 28);
                let mut attrs = Vec::with_capacity(acount);
                for k in 0..acount {
                    let a = p + 16 + astart + k * asize;
                    let ns_i = u32le(d, a);
                    let name_i = u32le(d, a + 4);
                    let raw_i = u32le(d, a + 8);
                    attrs.push(XmlAttr {
                        ns: if ns_i == 0xffff_ffff { String::new() } else { s(&pool, ns_i) },
                        name: s(&pool, name_i),
                        res_id: resmap.get(name_i as usize).copied().unwrap_or(0),
                        raw: if raw_i != 0xffff_ffff {
                            Some(s(&pool, raw_i))
                        } else if d[a + 15] == 3 {
                            Some(s(&pool, u32le(d, a + 16)))
                        } else {
                            None
                        },
                        ty: d[a + 15],
                        data: u32le(d, a + 16),
                    });
                }
                out.push(XmlEv::Start(name, attrs));
            }
            0x0103 => out.push(XmlEv::End(s(&pool, u32le(d, p + 20)))),
            _ => {}
        }
        p += size;
    }
    Ok(out)
}

fn attr_str(a: &XmlAttr, pool_lookup: bool) -> String {
    let _ = pool_lookup;
    match &a.raw {
        Some(r) => r.clone(),
        None => a.data.to_string(),
    }
}

pub fn parse_manifest(d: &[u8]) -> Result<Manifest, String> {
    let mut m = Manifest { package: String::new(), launcher: None, min_sdk: None, target_sdk: None, permissions: vec![], label: None, label_res: None };
    let mut cur_activity = String::new();
    let mut has_main = false;
    let mut has_launcher = false;
    let find = |attrs: &Vec<XmlAttr>, n: &str| -> Option<(String, u32, u8)> {
        attrs.iter().find(|a| a.name == n).map(|a| (attr_str(a, true), a.data, a.ty))
    };
    for ev in parse_xml(d)? {
        match ev {
            XmlEv::Start(name, attrs) => match name.as_str() {
                "manifest" => m.package = find(&attrs, "package").map(|v| v.0).unwrap_or_default(),
                "uses-sdk" => {
                    m.min_sdk = find(&attrs, "minSdkVersion").map(|v| v.1);
                    m.target_sdk = find(&attrs, "targetSdkVersion").map(|v| v.1);
                }
                "uses-permission" => {
                    if let Some(v) = find(&attrs, "name") {
                        m.permissions.push(v.0);
                    }
                }
                "application" => {
                    if let Some((s, data, ty)) = find(&attrs, "label") {
                        if ty == 1 {
                            m.label_res = Some(data);
                        } else {
                            m.label = Some(s);
                        }
                    }
                }
                "activity" | "activity-alias" => {
                    cur_activity = find(&attrs, "name").map(|v| v.0).unwrap_or_default();
                    has_main = false;
                    has_launcher = false;
                }
                "action" => {
                    if find(&attrs, "name").map(|v| v.0).as_deref() == Some("android.intent.action.MAIN") {
                        has_main = true;
                    }
                }
                "category" => {
                    if find(&attrs, "name").map(|v| v.0).as_deref() == Some("android.intent.category.LAUNCHER") {
                        has_launcher = true;
                    }
                }
                _ => {}
            },
            XmlEv::End(name) => {
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
        }
    }
    Ok(m)
}
