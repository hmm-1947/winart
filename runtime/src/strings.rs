use crate::runtime::{Kind, Runtime, R};

#[derive(Clone, Copy, PartialEq)]
enum Cs {
    Utf8,
    Latin1,
    Ascii,
    Utf16Le,
    Utf16Be,
    Utf16,
}

fn charset_named(n: &str) -> Option<Cs> {
    match n.to_ascii_uppercase().as_str() {
        "UTF-8" | "UTF8" => Some(Cs::Utf8),
        "ISO-8859-1" | "ISO8859_1" | "ISO_8859_1" | "LATIN1" | "L1" | "ISO-LATIN-1" => Some(Cs::Latin1),
        "US-ASCII" | "ASCII" => Some(Cs::Ascii),
        "UTF-16LE" => Some(Cs::Utf16Le),
        "UTF-16BE" => Some(Cs::Utf16Be),
        "UTF-16" => Some(Cs::Utf16),
        _ => None,
    }
}

fn decode(cs: Cs, b: &[u8]) -> Vec<u16> {
    match cs {
        Cs::Utf8 => String::from_utf8_lossy(b).encode_utf16().collect(),
        Cs::Latin1 => b.iter().map(|&x| x as u16).collect(),
        Cs::Ascii => b.iter().map(|&x| if x < 0x80 { x as u16 } else { 0xFFFD }).collect(),
        Cs::Utf16Le | Cs::Utf16Be | Cs::Utf16 => {
            let mut be = cs != Cs::Utf16Le;
            let mut s = b;
            if cs == Cs::Utf16 && s.len() >= 2 {
                if s[0] == 0xFF && s[1] == 0xFE {
                    be = false;
                    s = &s[2..];
                } else if s[0] == 0xFE && s[1] == 0xFF {
                    s = &s[2..];
                }
            }
            let mut out: Vec<u16> = s
                .chunks_exact(2)
                .map(|c| if be { ((c[0] as u16) << 8) | c[1] as u16 } else { ((c[1] as u16) << 8) | c[0] as u16 })
                .collect();
            if s.len() % 2 == 1 {
                out.push(0xFFFD);
            }
            out
        }
    }
}

fn encode(cs: Cs, u: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(u.len());
    match cs {
        Cs::Utf8 | Cs::Latin1 | Cs::Ascii => {
            let mut i = 0;
            while i < u.len() {
                let c = u[i];
                let mut cp = c as u32;
                let mut bad = false;
                if (0xD800..0xDC00).contains(&c) {
                    if i + 1 < u.len() && (0xDC00..0xE000).contains(&u[i + 1]) {
                        cp = 0x10000 + (((c as u32) - 0xD800) << 10) + (u[i + 1] as u32 - 0xDC00);
                        i += 1;
                    } else {
                        bad = true;
                    }
                } else if (0xDC00..0xE000).contains(&c) {
                    bad = true;
                }
                i += 1;
                if bad {
                    out.push(b'?');
                    continue;
                }
                match cs {
                    Cs::Utf8 => {
                        let mut buf = [0u8; 4];
                        out.extend_from_slice(char::from_u32(cp).unwrap_or('?').encode_utf8(&mut buf).as_bytes());
                    }
                    Cs::Latin1 => out.push(if cp <= 0xFF { cp as u8 } else { b'?' }),
                    _ => out.push(if cp < 0x80 { cp as u8 } else { b'?' }),
                }
            }
        }
        Cs::Utf16Le => {
            for &c in u {
                out.push(c as u8);
                out.push((c >> 8) as u8);
            }
        }
        Cs::Utf16Be | Cs::Utf16 => {
            if cs == Cs::Utf16 {
                out.push(0xFE);
                out.push(0xFF);
            }
            for &c in u {
                out.push((c >> 8) as u8);
                out.push(c as u8);
            }
        }
    }
    out
}

fn hash_of(u: &[u16]) -> i32 {
    let mut h: i32 = 0;
    for &c in u {
        h = h.wrapping_mul(31).wrapping_add(c as i32);
    }
    h
}

fn find(h: &[u16], n: &[u16], from: usize) -> Option<usize> {
    if n.is_empty() {
        return Some(from.min(h.len()));
    }
    if n.len() > h.len() || from > h.len() - n.len() {
        return None;
    }
    (from..=h.len() - n.len()).find(|&i| &h[i..i + n.len()] == n)
}

fn index_of_str(h: &[u16], n: &[u16], from: i32) -> i32 {
    let len = h.len() as i32;
    let from = from.max(0);
    if from >= len {
        return if n.is_empty() { len } else { -1 };
    }
    match find(h, n, from as usize) {
        Some(i) => i as i32,
        None => -1,
    }
}

fn last_index_of_str(h: &[u16], n: &[u16], from: i32) -> i32 {
    let right = h.len() as i32 - n.len() as i32;
    let from = from.min(right);
    if from < 0 {
        return -1;
    }
    if n.is_empty() {
        return from;
    }
    let mut i = from as usize;
    loop {
        if &h[i..i + n.len()] == n {
            return i as i32;
        }
        if i == 0 {
            return -1;
        }
        i -= 1;
    }
}

fn units_of_cp(cp: i32) -> Option<Vec<u16>> {
    if !(0..=0x10FFFF).contains(&cp) {
        return None;
    }
    if cp < 0x10000 {
        return Some(vec![cp as u16]);
    }
    let c = cp as u32 - 0x10000;
    Some(vec![0xD800 + (c >> 10) as u16, 0xDC00 + (c & 0x3FF) as u16])
}

fn index_of_cp(h: &[u16], cp: i32, from: i32) -> i32 {
    match units_of_cp(cp) {
        None => -1,
        Some(n) => {
            let len = h.len() as i32;
            let from = from.max(0);
            if from >= len {
                return -1;
            }
            match find(h, &n, from as usize) {
                Some(i) => i as i32,
                None => -1,
            }
        }
    }
}

fn last_index_of_cp(h: &[u16], cp: i32, from: i32) -> i32 {
    match units_of_cp(cp) {
        None => -1,
        Some(n) => {
            if h.is_empty() || from < 0 {
                return -1;
            }
            last_index_of_str(h, &n, from.min(h.len() as i32 - 1))
        }
    }
}

fn simple_case(c: u16, upper: bool) -> u16 {
    match char::from_u32(c as u32) {
        Some(ch) => {
            let mut a = ch.to_uppercase();
            let mut b = ch.to_lowercase();
            let r = if upper { (a.next(), a.next()) } else { (b.next(), b.next()) };
            match r {
                (Some(x), None) if (x as u32) < 0x10000 => x as u16,
                _ => c,
            }
        }
        None => c,
    }
}

fn up(c: u16) -> u16 {
    simple_case(c, true)
}

fn low(c: u16) -> u16 {
    simple_case(c, false)
}

fn map_case(u: &[u16], upper: bool) -> Vec<u16> {
    match String::from_utf16(u) {
        Ok(s) => {
            let r = if upper { s.to_uppercase() } else { s.to_lowercase() };
            r.encode_utf16().collect()
        }
        Err(_) => u.iter().map(|&c| simple_case(c, upper)).collect(),
    }
}

impl Runtime {
    fn us(&self, r: u32) -> &[u16] {
        match &self.heap[r as usize].kind {
            Kind::Str(u) => u,
            _ => &[],
        }
    }

    fn is_str(&self, r: u32) -> bool {
        r != 0 && matches!(self.heap[r as usize].kind, Kind::Str(_))
    }

    pub(crate) fn bytes_of(&self, r: u32) -> Vec<u8> {
        match &self.heap[r as usize].kind {
            Kind::Array(v) => v.iter().map(|&x| x as u8).collect(),
            _ => vec![],
        }
    }

    pub(crate) fn arr_len(&self, r: u32) -> usize {
        match &self.heap[r as usize].kind {
            Kind::Array(v) => v.len(),
            _ => 0,
        }
    }

    fn sioobe(&mut self, msg: String) -> R<u64> {
        Err(self.throw_msg("Ljava/lang/StringIndexOutOfBoundsException;", &msg))
    }

    fn ret_units(&mut self, u: Vec<u16>) -> R<u64> {
        Ok(self.new_string(u) as u64)
    }

    fn charset_of_obj(&mut self, o: u32) -> Option<Cs> {
        if o == 0 {
            return None;
        }
        let c = self.heap[o as usize].class;
        let slot = *self.classes[c].inst_idx.get("name")?;
        let n = *self.heap[o as usize].fields.get(slot)? as u32;
        if !self.is_str(n) {
            return None;
        }
        charset_named(&String::from_utf16_lossy(self.us(n)))
    }

    fn charset_of_name(&self, o: u32) -> Option<Cs> {
        if !self.is_str(o) {
            return None;
        }
        charset_named(&String::from_utf16_lossy(self.us(o)))
    }

    fn region_check(&mut self, len: usize, off: i32, cnt: i32) -> Option<R<u64>> {
        if (off | cnt) < 0 || cnt as i64 > len as i64 - off as i64 {
            return Some(self.sioobe(format!("length={}; regionStart={}; regionLength={}", len, off, cnt)));
        }
        None
    }

    pub(crate) fn string_native(&mut self, key: &str, a: &[u32]) -> Option<R<u64>> {
        let this = a.first().copied().unwrap_or(0);
        let r: R<u64> = match key {
            "length()I" => Ok(self.us(this).len() as u64),
            "isEmpty()Z" => Ok(self.us(this).is_empty() as u64),
            "charAt(I)C" => {
                let i = a[1] as i32;
                let u = self.us(this);
                if i >= 0 && (i as usize) < u.len() {
                    Ok(u[i as usize] as u64)
                } else {
                    let l = u.len();
                    self.sioobe(format!("length={}; index={}", l, i))
                }
            }
            "equals(Ljava/lang/Object;)Z" => Ok((self.is_str(a[1]) && self.us(this) == self.us(a[1])) as u64),
            "hashCode()I" => {
                let (_, _, _, hs) = self.str_layout();
                if let Some(s) = hs {
                    if let Some(&v) = self.heap[this as usize].fields.get(s) {
                        if v != 0 {
                            return Some(Ok(v));
                        }
                    }
                }
                let h = hash_of(self.us(this)) as u32 as u64;
                if let Some(s) = hs {
                    if let Some(f) = self.heap[this as usize].fields.get_mut(s) {
                        *f = h;
                    }
                }
                Ok(h)
            }
            "compareTo(Ljava/lang/String;)I" | "compareToIgnoreCase(Ljava/lang/String;)I" => {
                if a[1] == 0 {
                    return Some(Err(self.npe()));
                }
                let ic = key.starts_with("compareToI");
                let (x, y) = (self.us(this), self.us(a[1]));
                let n = x.len().min(y.len());
                let mut res = x.len() as i32 - y.len() as i32;
                for i in 0..n {
                    if x[i] != y[i] {
                        if !ic {
                            res = x[i] as i32 - y[i] as i32;
                            break;
                        }
                        let (p, q) = (up(x[i]), up(y[i]));
                        if p != q {
                            let (p, q) = (low(p), low(q));
                            if p != q {
                                res = p as i32 - q as i32;
                                break;
                            }
                        }
                    }
                }
                Ok(res as u32 as u64)
            }
            "equalsIgnoreCase(Ljava/lang/String;)Z" => {
                if !self.is_str(a[1]) {
                    return Some(Ok(0));
                }
                let (x, y) = (self.us(this), self.us(a[1]));
                let eq = x.len() == y.len() && x.iter().zip(y.iter()).all(|(&p, &q)| p == q || up(p) == up(q) || low(up(p)) == low(up(q)));
                Ok(eq as u64)
            }
            "getCharsNoCheck(II[CI)V" => {
                let (s, e, idx) = (a[1] as i32, a[2] as i32, a[4] as i32);
                if a[3] == 0 {
                    return Some(Err(self.npe()));
                }
                let u = self.us(this);
                if s < 0 || e < s || e as usize > u.len() || idx < 0 || idx as usize + (e - s) as usize > self.arr_len(a[3]) {
                    return Some(Err(self.throw_new("Ljava/lang/ArrayIndexOutOfBoundsException;")));
                }
                let part: Vec<u64> = u[s as usize..e as usize].iter().map(|&c| c as u64).collect();
                if let Kind::Array(v) = &mut self.heap[a[3] as usize].kind {
                    v[idx as usize..idx as usize + part.len()].copy_from_slice(&part);
                }
                Ok(0)
            }
            "fastSubstring(II)Ljava/lang/String;" | "substring(II)Ljava/lang/String;" => {
                let (s, e) = if key.starts_with("fast") { (a[1] as i32, (a[1] as i32).wrapping_add(a[2] as i32)) } else { (a[1] as i32, a[2] as i32) };
                let len = self.us(this).len() as i32;
                if s < 0 || e > len || s > e {
                    return Some(self.sioobe(format!("begin {}, end {}, length {}", s, e, len)));
                }
                if s == 0 && e == len {
                    return Some(Ok(this as u64));
                }
                let u = self.us(this)[s as usize..e as usize].to_vec();
                self.ret_units(u)
            }
            "substring(I)Ljava/lang/String;" => {
                let s = a[1] as i32;
                let len = self.us(this).len() as i32;
                if s < 0 || s > len {
                    return Some(self.sioobe(format!("begin {}, end {}, length {}", s, len, len)));
                }
                if s == 0 {
                    return Some(Ok(this as u64));
                }
                let u = self.us(this)[s as usize..].to_vec();
                self.ret_units(u)
            }
            "concat(Ljava/lang/String;)Ljava/lang/String;" => {
                if a[1] == 0 {
                    return Some(Err(self.npe()));
                }
                if self.us(a[1]).is_empty() {
                    return Some(Ok(this as u64));
                }
                let mut u = self.us(this).to_vec();
                u.extend_from_slice(self.us(a[1]));
                self.ret_units(u)
            }
            "doReplace(CC)Ljava/lang/String;" | "replace(CC)Ljava/lang/String;" => {
                let (o, n) = (a[1] as u16, a[2] as u16);
                if !self.us(this).contains(&o) {
                    return Some(Ok(this as u64));
                }
                let u: Vec<u16> = self.us(this).iter().map(|&c| if c == o { n } else { c }).collect();
                self.ret_units(u)
            }
            "replace(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Ljava/lang/String;" => {
                if a[1] == 0 || a[2] == 0 {
                    return Some(Err(self.npe()));
                }
                if !self.is_str(a[1]) || !self.is_str(a[2]) {
                    return None;
                }
                let (h, t, w) = (self.us(this), self.us(a[1]), self.us(a[2]));
                let mut out = Vec::with_capacity(h.len());
                let mut hit = false;
                if t.is_empty() {
                    out.extend_from_slice(w);
                    for &c in h {
                        out.push(c);
                        out.extend_from_slice(w);
                    }
                    hit = true;
                } else {
                    let mut i = 0;
                    while i < h.len() {
                        if i + t.len() <= h.len() && &h[i..i + t.len()] == t {
                            out.extend_from_slice(w);
                            i += t.len();
                            hit = true;
                        } else {
                            out.push(h[i]);
                            i += 1;
                        }
                    }
                }
                if !hit {
                    return Some(Ok(this as u64));
                }
                self.ret_units(out)
            }
            "toCharArray()[C" => {
                let v: Vec<u64> = self.us(this).iter().map(|&c| c as u64).collect();
                Ok(self.new_array("[C", v) as u64)
            }
            "intern()Ljava/lang/String;" => match String::from_utf16(self.us(this)) {
                Ok(s) => {
                    if let Some(&r) = self.interned.get(&s) {
                        return Some(Ok(r as u64));
                    }
                    self.interned.insert(s, this);
                    Ok(this as u64)
                }
                Err(_) => Ok(this as u64),
            },
            "doRepeat(I)Ljava/lang/String;" => {
                let n = a[1] as i32;
                if n < 0 {
                    return Some(Err(self.throw_msg("Ljava/lang/IllegalArgumentException;", &format!("count is negative: {}", n))));
                }
                let u = self.us(this).repeat(n as usize);
                self.ret_units(u)
            }
            "fillBytesLatin1([BI)V" | "fillBytesUTF16([BI)V" => {
                let utf16 = key.contains("UTF16");
                let idx = a[2] as usize;
                let u = self.us(this).to_vec();
                if let Kind::Array(v) = &mut self.heap[a[1] as usize].kind {
                    for (i, &c) in u.iter().enumerate() {
                        if utf16 {
                            if idx + 2 * i + 1 < v.len() {
                                v[idx + 2 * i] = (c & 0xff) as u64;
                                v[idx + 2 * i + 1] = (c >> 8) as u64;
                            }
                        } else if idx + i < v.len() {
                            v[idx + i] = (c & 0xff) as u64;
                        }
                    }
                }
                Ok(0)
            }
            "indexOf(I)I" => Ok(index_of_cp(self.us(this), a[1] as i32, 0) as u32 as u64),
            "indexOf(II)I" => Ok(index_of_cp(self.us(this), a[1] as i32, a[2] as i32) as u32 as u64),
            "lastIndexOf(I)I" => {
                let l = self.us(this).len() as i32;
                Ok(last_index_of_cp(self.us(this), a[1] as i32, l - 1) as u32 as u64)
            }
            "lastIndexOf(II)I" => Ok(last_index_of_cp(self.us(this), a[1] as i32, a[2] as i32) as u32 as u64),
            "indexOf(Ljava/lang/String;)I" | "indexOf(Ljava/lang/String;I)I" => {
                if a[1] == 0 {
                    return Some(Err(self.npe()));
                }
                let from = if key.contains(";I)") { a[2] as i32 } else { 0 };
                Ok(index_of_str(self.us(this), self.us(a[1]), from) as u32 as u64)
            }
            "lastIndexOf(Ljava/lang/String;)I" | "lastIndexOf(Ljava/lang/String;I)I" => {
                if a[1] == 0 {
                    return Some(Err(self.npe()));
                }
                let from = if key.contains(";I)") { a[2] as i32 } else { i32::MAX };
                Ok(last_index_of_str(self.us(this), self.us(a[1]), from) as u32 as u64)
            }
            "contains(Ljava/lang/CharSequence;)Z" => {
                if a[1] == 0 {
                    return Some(Err(self.npe()));
                }
                if !self.is_str(a[1]) {
                    return None;
                }
                Ok(find(self.us(this), self.us(a[1]), 0).is_some() as u64)
            }
            "startsWith(Ljava/lang/String;)Z" | "startsWith(Ljava/lang/String;I)Z" => {
                if a[1] == 0 {
                    return Some(Err(self.npe()));
                }
                let off = if key.contains(";I)") { a[2] as i32 } else { 0 };
                let (h, p) = (self.us(this), self.us(a[1]));
                Ok((off >= 0 && off as usize <= h.len() && h[off as usize..].starts_with(p)) as u64)
            }
            "endsWith(Ljava/lang/String;)Z" => {
                if a[1] == 0 {
                    return Some(Err(self.npe()));
                }
                Ok(self.us(this).ends_with(self.us(a[1])) as u64)
            }
            "trim()Ljava/lang/String;" => {
                let u = self.us(this);
                let s = u.iter().position(|&c| c > 0x20).unwrap_or(u.len());
                let e = u.iter().rposition(|&c| c > 0x20).map(|i| i + 1).unwrap_or(s);
                if s == 0 && e == u.len() {
                    return Some(Ok(this as u64));
                }
                let v = u[s..e].to_vec();
                self.ret_units(v)
            }
            "toUpperCase()Ljava/lang/String;" | "toUpperCase(Ljava/util/Locale;)Ljava/lang/String;" | "toLowerCase()Ljava/lang/String;" | "toLowerCase(Ljava/util/Locale;)Ljava/lang/String;" => {
                let upper = key.starts_with("toUpper");
                if key.contains("Locale") && a[1] == 0 {
                    return Some(Err(self.npe()));
                }
                let m = map_case(self.us(this), upper);
                if m == self.us(this) {
                    return Some(Ok(this as u64));
                }
                self.ret_units(m)
            }
            "getBytes()[B" => {
                let b = encode(Cs::Utf8, self.us(this));
                Ok(self.new_array("[B", b.into_iter().map(|x| x as u64).collect()) as u64)
            }
            "getBytes(Ljava/lang/String;)[B" | "getBytes(Ljava/nio/charset/Charset;)[B" => {
                if a[1] == 0 {
                    return Some(Err(self.npe()));
                }
                let cs = if key.contains("Charset") { self.charset_of_obj(a[1]) } else { self.charset_of_name(a[1]) };
                let cs = cs?;
                let b = encode(cs, self.us(this));
                Ok(self.new_array("[B", b.into_iter().map(|x| x as u64).collect()) as u64)
            }
            _ => return None,
        };
        Some(r)
    }

    pub(crate) fn factory_native(&mut self, key: &str, a: &[u32]) -> Option<R<u64>> {
        let r: R<u64> = match key {
            "newEmptyString()Ljava/lang/String;" => self.ret_units(vec![]),
            "newStringFromString(Ljava/lang/String;)Ljava/lang/String;" => {
                if a[0] == 0 {
                    return Some(Err(self.npe()));
                }
                let u = self.us(a[0]).to_vec();
                self.ret_units(u)
            }
            "newStringFromChars([C)Ljava/lang/String;" | "newStringFromChars([CII)Ljava/lang/String;" | "newStringFromChars(II[C)Ljava/lang/String;" => {
                let (arr, off, cnt) = match key {
                    "newStringFromChars([C)Ljava/lang/String;" => (a[0], 0, if a[0] == 0 { 0 } else { self.arr_len(a[0]) as i32 }),
                    "newStringFromChars([CII)Ljava/lang/String;" => (a[0], a[1] as i32, a[2] as i32),
                    _ => (a[2], a[0] as i32, a[1] as i32),
                };
                if arr == 0 {
                    return Some(Err(self.npe()));
                }
                let len = self.arr_len(arr);
                if let Some(e) = self.region_check(len, off, cnt) {
                    return Some(e);
                }
                let u: Vec<u16> = match &self.heap[arr as usize].kind {
                    Kind::Array(v) => v[off as usize..(off + cnt) as usize].iter().map(|&x| x as u16).collect(),
                    _ => vec![],
                };
                self.ret_units(u)
            }
            "newStringFromUtf16Bytes([BII)Ljava/lang/String;" => {
                if a[0] == 0 {
                    return Some(Err(self.npe()));
                }
                let b = self.bytes_of(a[0]);
                let (off, cnt) = (a[1] as i32, a[2] as i32);
                if (off | cnt) < 0 || off as i64 + 2 * cnt as i64 > b.len() as i64 {
                    return Some(self.sioobe(format!("length={}; regionStart={}; regionLength={}", b.len() / 2, off, cnt)));
                }
                let u = decode(Cs::Utf16Le, &b[off as usize..(off + 2 * cnt) as usize]);
                self.ret_units(u)
            }
            "newStringFromBytes([BB)Ljava/lang/String;" => {
                if a[0] == 0 {
                    return Some(Err(self.npe()));
                }
                let b = self.bytes_of(a[0]);
                let cs = if a[1] as u8 == 0 { Cs::Latin1 } else { Cs::Utf16Le };
                self.ret_units(decode(cs, &b))
            }
            "newStringFromBytes([BI)Ljava/lang/String;" => {
                if a[0] == 0 {
                    return Some(Err(self.npe()));
                }
                let hi = (a[1] as u16) << 8;
                let u: Vec<u16> = self.bytes_of(a[0]).iter().map(|&x| hi | x as u16).collect();
                self.ret_units(u)
            }
            "newStringFromBytes([BIII)Ljava/lang/String;" => {
                if a[0] == 0 {
                    return Some(Err(self.npe()));
                }
                let b = self.bytes_of(a[0]);
                let (off, cnt) = (a[2] as i32, a[3] as i32);
                if let Some(e) = self.region_check(b.len(), off, cnt) {
                    return Some(e);
                }
                let hi = (a[1] as u16) << 8;
                let u: Vec<u16> = b[off as usize..(off + cnt) as usize].iter().map(|&x| hi | x as u16).collect();
                self.ret_units(u)
            }
            "newStringFromBytes([B)Ljava/lang/String;"
            | "newStringFromBytes([BII)Ljava/lang/String;"
            | "newStringFromUtf8Bytes([BII)Ljava/lang/String;"
            | "newStringFromBytes([BLjava/lang/String;)Ljava/lang/String;"
            | "newStringFromBytes([BLjava/nio/charset/Charset;)Ljava/lang/String;"
            | "newStringFromBytes([BIILjava/lang/String;)Ljava/lang/String;"
            | "newStringFromBytes([BIILjava/nio/charset/Charset;)Ljava/lang/String;" => {
                if a[0] == 0 {
                    return Some(Err(self.npe()));
                }
                let b = self.bytes_of(a[0]);
                let ranged = key.contains("II");
                let (off, cnt) = if ranged { (a[1] as i32, a[2] as i32) } else { (0, b.len() as i32) };
                let extra = if ranged { a.get(3) } else { a.get(1) }.copied().unwrap_or(0);
                let cs = if key.contains("Charset;)") {
                    if extra == 0 {
                        return Some(Err(self.npe()));
                    }
                    self.charset_of_obj(extra)?
                } else if key.contains("String;)Ljava/lang/String;") {
                    if extra == 0 {
                        return Some(Err(self.npe()));
                    }
                    self.charset_of_name(extra)?
                } else {
                    Cs::Utf8
                };
                if let Some(e) = self.region_check(b.len(), off, cnt) {
                    return Some(e);
                }
                self.ret_units(decode(cs, &b[off as usize..(off + cnt) as usize]))
            }
            _ => return None,
        };
        Some(r)
    }
}
