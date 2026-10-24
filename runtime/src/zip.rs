pub struct Entry {
    pub name: String,
    pub method: u16,
    pub comp_size: u32,
    pub size: u32,
    pub local_off: u32,
}

pub struct Zip {
    data: Vec<u8>,
    pub entries: Vec<Entry>,
}

fn u16le(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}

fn u32le(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

impl Zip {
    pub fn open(path: &str) -> Result<Zip, String> {
        let data = std::fs::read(path).map_err(|e| format!("{}: {}", path, e))?;
        Zip::from_bytes(data)
    }

    pub fn from_bytes(data: Vec<u8>) -> Result<Zip, String> {
        if data.len() < 22 {
            return Err("zip too small".into());
        }
        let mut eocd = None;
        let lo = data.len().saturating_sub(22 + 65535);
        let mut i = data.len() - 22;
        loop {
            if u32le(&data, i) == 0x06054b50 {
                eocd = Some(i);
                break;
            }
            if i == lo {
                break;
            }
            i -= 1;
        }
        let eocd = eocd.ok_or("no end of central directory")?;
        let count = u16le(&data, eocd + 10) as usize;
        let mut p = u32le(&data, eocd + 16) as usize;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            if p + 46 > data.len() || u32le(&data, p) != 0x02014b50 {
                return Err("bad central directory".into());
            }
            let method = u16le(&data, p + 10);
            let comp_size = u32le(&data, p + 20);
            let size = u32le(&data, p + 24);
            let nl = u16le(&data, p + 28) as usize;
            let el = u16le(&data, p + 30) as usize;
            let cl = u16le(&data, p + 32) as usize;
            let local_off = u32le(&data, p + 42);
            let name = String::from_utf8_lossy(&data[p + 46..p + 46 + nl]).into_owned();
            entries.push(Entry { name, method, comp_size, size, local_off });
            p += 46 + nl + el + cl;
        }
        Ok(Zip { data, entries })
    }

    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        let e = self.entries.iter().find(|e| e.name == name)?;
        let o = e.local_off as usize;
        if u32le(&self.data, o) != 0x04034b50 {
            return None;
        }
        let nl = u16le(&self.data, o + 26) as usize;
        let el = u16le(&self.data, o + 28) as usize;
        let start = o + 30 + nl + el;
        let raw = &self.data[start..start + e.comp_size as usize];
        match e.method {
            0 => Some(raw.to_vec()),
            8 => miniz_oxide::inflate::decompress_to_vec(raw).ok(),
            _ => None,
        }
    }
}
