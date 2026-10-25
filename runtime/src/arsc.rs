use crate::axml::{parse_pool, u16le, u32le};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct ResValue {
    pub ty: u8,
    pub data: u32,
}

struct Entry {
    value: Option<ResValue>,
    default: bool,
}

pub struct Arsc {
    pub package: String,
    strings: Vec<String>,
    entries: HashMap<u32, Entry>,
    names: HashMap<(String, String), u32>,
}

impl Arsc {
    pub fn lookup(&self, mut id: u32) -> Option<ResValue> {
        for _ in 0..16 {
            let v = self.entries.get(&id)?.value?;
            if v.ty == 1 {
                id = v.data;
                continue;
            }
            return Some(v);
        }
        None
    }

    pub fn string(&self, v: ResValue) -> Option<String> {
        if v.ty == 3 {
            self.strings.get(v.data as usize).cloned()
        } else {
            None
        }
    }

    pub fn find(&self, name: &str, ty: &str) -> Option<u32> {
        self.names.get(&(ty.to_string(), name.to_string())).copied()
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }
}

pub fn parse(d: &[u8]) -> Result<Arsc, String> {
    if d.len() < 12 || u16le(d, 0) != 0x0002 {
        return Err("not resources.arsc".into());
    }
    let total = (u32le(d, 4) as usize).min(d.len());
    let mut a = Arsc { package: String::new(), strings: vec![], entries: HashMap::new(), names: HashMap::new() };
    let mut p = u16le(d, 2);
    while p + 8 <= total {
        let ty = u16le(d, p);
        let size = u32le(d, p + 4) as usize;
        if size == 0 {
            break;
        }
        match ty {
            0x0001 => a.strings = parse_pool(d, p),
            0x0200 => parse_package(d, p, size, &mut a)?,
            _ => {}
        }
        p += size;
    }
    Ok(a)
}

fn parse_package(d: &[u8], p: usize, size: usize, a: &mut Arsc) -> Result<(), String> {
    let hdr = u16le(d, p + 2);
    let pkg_id = u32le(d, p + 8);
    let units: Vec<u16> = (0..128).map(|k| u16le(d, p + 12 + k * 2) as u16).take_while(|&u| u != 0).collect();
    let pkg_name = String::from_utf16_lossy(&units);
    let types = parse_pool(d, p + u32le(d, p + 268) as usize);
    let keys = parse_pool(d, p + u32le(d, p + 276) as usize);
    if pkg_id == 0x7f || a.package.is_empty() {
        a.package = pkg_name;
    }
    let end = (p + size).min(d.len());
    let mut q = p + hdr;
    while q + 8 <= end {
        let ty = u16le(d, q);
        let csize = u32le(d, q + 4) as usize;
        if csize == 0 {
            break;
        }
        if ty == 0x0201 {
            parse_type(d, q, pkg_id, &types, &keys, a);
        }
        q += csize;
    }
    Ok(())
}

fn parse_type(d: &[u8], q: usize, pkg_id: u32, types: &[String], keys: &[String], a: &mut Arsc) {
    let type_id = d[q + 8] as u32;
    let flags = d[q + 9];
    let count = u32le(d, q + 12) as usize;
    let entries_start = u32le(d, q + 16) as usize;
    let hdr = u16le(d, q + 2);
    let cfg_size = u32le(d, q + 20) as usize;
    let cfg_end = (q + 20 + cfg_size).min(q + hdr);
    let default = d[(q + 24).min(cfg_end)..cfg_end].iter().all(|&b| b == 0);
    let type_name = types.get(type_id as usize - 1).cloned().unwrap_or_default();
    for i in 0..count {
        let (idx, off) = if flags & 1 != 0 {
            let v = u32le(d, q + hdr + i * 4);
            ((v & 0xffff) as u32, ((v >> 16) as usize) * 4)
        } else if flags & 2 != 0 {
            let o = u16le(d, q + hdr + i * 2);
            if o == 0xffff {
                continue;
            }
            (i as u32, o * 4)
        } else {
            let o = u32le(d, q + hdr + i * 4);
            if o == 0xffff_ffff {
                continue;
            }
            (i as u32, o as usize)
        };
        let e = q + entries_start + off;
        if e + 8 > d.len() {
            continue;
        }
        let eflags = u16le(d, e + 2);
        let key = u32le(d, e + 4) as usize;
        let id = (pkg_id << 24) | (type_id << 16) | idx;
        let value = if eflags & 1 != 0 {
            None
        } else {
            Some(ResValue { ty: d[e + 11], data: u32le(d, e + 12) })
        };
        if let Some(k) = keys.get(key) {
            a.names.entry((type_name.clone(), k.clone())).or_insert(id);
        }
        let replace = match a.entries.get(&id) {
            None => true,
            Some(old) => !old.default && default,
        };
        if replace {
            a.entries.insert(id, Entry { value, default });
        }
    }
}
