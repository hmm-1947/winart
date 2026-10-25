use crate::runtime::{hook_for, Kind, Obj, Runtime, HOOK_BITS, HOOK_CHAR, HOOK_CLASS, HOOK_FACTORY, HOOK_MATCHER, HOOK_MATH, HOOK_OBJECT, HOOK_PATTERN, HOOK_REFERENCE, HOOK_REGISTRY, HOOK_STRING, HOOK_SYSTEM, HOOK_THREAD, HOOK_UTF16, HOOK_VMRUNTIME, R};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static HASH_SEED: AtomicU32 = AtomicU32::new(0x2545_F491);
static START: OnceLock<Instant> = OnceLock::new();

fn next_hash() -> u32 {
    let mut x = HASH_SEED.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    HASH_SEED.store(x, Ordering::Relaxed);
    let h = x & 0x0FFF_FFFF;
    if h == 0 {
        1
    } else {
        h
    }
}

fn dbl(a: &[u32], i: usize) -> f64 {
    f64::from_bits((a[i] as u64) | ((a[i + 1] as u64) << 32))
}

fn java_pow(x: f64, y: f64) -> f64 {
    if y == 0.0 {
        return 1.0;
    }
    if y.is_nan() {
        return f64::NAN;
    }
    if x.abs() == 1.0 && y.is_infinite() {
        return f64::NAN;
    }
    x.powf(y)
}

fn ieee_remainder(x: f64, y: f64) -> f64 {
    if x.is_nan() || y.is_nan() || x.is_infinite() || y == 0.0 {
        return f64::NAN;
    }
    if y.is_infinite() {
        return x;
    }
    let mut r = x % y;
    let ay = y.abs();
    let twice = r.abs() * 2.0;
    if twice > ay || (twice == ay && ((x - r) / y) % 2.0 != 0.0) {
        r -= ay.copysign(r);
    }
    r
}

fn java_name(desc: &str) -> String {
    match desc {
        "I" => "int".into(),
        "J" => "long".into(),
        "S" => "short".into(),
        "B" => "byte".into(),
        "C" => "char".into(),
        "Z" => "boolean".into(),
        "F" => "float".into(),
        "D" => "double".into(),
        "V" => "void".into(),
        _ if desc.starts_with('[') => desc.replace('/', "."),
        _ => desc.trim_start_matches('L').trim_end_matches(';').replace('/', "."),
    }
}

fn simple_name(desc: &str) -> String {
    if let Some(c) = desc.strip_prefix('[') {
        return format!("{}[]", simple_name(c));
    }
    let n = java_name(desc);
    let s = n.rsplit('.').next().unwrap_or("").to_string();
    match s.rfind('$') {
        Some(i) => s[i + 1..].trim_start_matches(|c: char| c.is_ascii_digit()).to_string(),
        None => s,
    }
}

fn is_prim_desc(d: &str) -> bool {
    d.len() == 1
}

impl Runtime {
    pub(crate) fn hook_native(&mut self, h: u8, key: &str, a: &[u32]) -> Option<R<u64>> {
        match h {
            HOOK_STRING => self.string_native(key, a),
            HOOK_FACTORY => self.factory_native(key, a),
            HOOK_OBJECT => self.object_native(key, a),
            HOOK_CLASS => self.class_native(key, a),
            HOOK_SYSTEM => self.system_native(key, a),
            HOOK_MATH => math_native(key, a),
            HOOK_BITS => bits_native(key, a),
            HOOK_CHAR => char_native(key, a),
            HOOK_THREAD => self.thread_native(key, a),
            HOOK_REGISTRY => self.registry_native(key),
            HOOK_PATTERN => self.pattern_native(key, a),
            HOOK_MATCHER => self.matcher_native(key, a),
            HOOK_REFERENCE => self.reference_native(key, a),
            HOOK_VMRUNTIME => vmruntime_native(key, a),
            HOOK_UTF16 => match key {
                "isBigEndian()Z" => Some(Ok(0)),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn hooked_native(&mut self, full: &str, a: &[u32]) -> Option<R<u64>> {
        let (cls, key) = full.split_once("->")?;
        let h = hook_for(cls);
        if h == 0 {
            return None;
        }
        self.hook_native(h, key, a)
    }

    pub(crate) fn identity_hash(&mut self, r: u32) -> u32 {
        if r == 0 {
            return 0;
        }
        let o = self.load("Ljava/lang/Object;");
        let slot = match self.classes[o].inst_idx.get("shadow$_monitor_") {
            Some(&s) => s,
            None => return (r.wrapping_mul(0x9E37_79B1) >> 4) & 0x0FFF_FFFF,
        };
        let f = &mut self.heap[r as usize].fields;
        if f.len() <= slot {
            f.resize(slot + 1, 0);
        }
        let lw = f[slot] as u32;
        if lw & 0xC000_0000 == 0x8000_0000 {
            return lw & 0x0FFF_FFFF;
        }
        let h = next_hash();
        f[slot] = (0x8000_0000 | h) as u64;
        h
    }

    fn clone_obj(&mut self, r: u32) -> u32 {
        let src = &self.heap[r as usize];
        let kind = match &src.kind {
            Kind::Plain => Kind::Plain,
            Kind::Str(u) => Kind::Str(u.clone()),
            Kind::Array(v) => Kind::Array(v.clone()),
            Kind::Class(c) => Kind::Class(*c),
        };
        let class = src.class;
        let mut fields = src.fields.clone();
        let o = self.load("Ljava/lang/Object;");
        if let Some(&slot) = self.classes[o].inst_idx.get("shadow$_monitor_") {
            if slot < fields.len() {
                fields[slot] = 0;
            }
        }
        self.alloc(Obj { class, fields, kind })
    }

    fn object_native(&mut self, key: &str, a: &[u32]) -> Option<R<u64>> {
        let this = a.first().copied().unwrap_or(0);
        let r: R<u64> = match key {
            "getClass()Ljava/lang/Class;" => {
                let c = self.heap[this as usize].class;
                Ok(self.class_obj(c) as u64)
            }
            "hashCode()I" => Ok(self.identity_hash(this) as u64),
            "identityHashCode(Ljava/lang/Object;)I" | "identityHashCodeNative(Ljava/lang/Object;)I" => Ok(self.identity_hash(this) as u64),
            "internalClone()Ljava/lang/Object;" => Ok(self.clone_obj(this) as u64),
            "clone()Ljava/lang/Object;" => {
                if matches!(self.heap[this as usize].kind, Kind::Array(_)) {
                    Ok(self.clone_obj(this) as u64)
                } else {
                    return None;
                }
            }
            "notify()V" | "notifyAll()V" | "wait(JI)V" => Ok(0),
            _ => return None,
        };
        Some(r)
    }

    fn class_of(&self, r: u32) -> Option<usize> {
        if r == 0 {
            return None;
        }
        match self.heap[r as usize].kind {
            Kind::Class(c) => Some(c),
            _ => None,
        }
    }

    fn assignable(&mut self, from: usize, to: usize) -> bool {
        if from == to {
            return true;
        }
        let (fname, tname) = (self.classes[from].name.clone(), self.classes[to].name.clone());
        if fname.starts_with('[') || tname.starts_with('[') {
            if tname == "Ljava/lang/Object;" {
                return !is_prim_desc(&fname);
            }
            if fname.starts_with('[') && tname.starts_with('[') {
                let (fc, tc) = (&fname[1..], &tname[1..]);
                if is_prim_desc(fc) || is_prim_desc(tc) {
                    return fc == tc;
                }
                let (fc, tc) = (fc.to_string(), tc.to_string());
                let (f2, t2) = (self.load(&fc), self.load(&tc));
                return self.assignable(f2, t2);
            }
            return tname == "Ljava/lang/Cloneable;" || tname == "Ljava/io/Serializable;";
        }
        self.is_sub(from, to)
    }

    fn class_native(&mut self, key: &str, a: &[u32]) -> Option<R<u64>> {
        if key == "classForName(Ljava/lang/String;ZLjava/lang/ClassLoader;)Ljava/lang/Class;" {
            if a[0] == 0 {
                return Some(Err(self.npe()));
            }
            let n = self.string_value(a[0]);
            let desc = if n.starts_with('[') { n.replace('.', "/") } else { format!("L{};", n.replace('.', "/")) };
            let known = self.defs.contains_key(&desc) || (desc.starts_with('[') && !desc.contains('.'));
            if !known {
                return Some(Err(self.throw_msg("Ljava/lang/ClassNotFoundException;", &n)));
            }
            let c = self.load(&desc);
            if a[1] & 1 != 0 {
                if let Err(e) = self.ensure_init(c) {
                    return Some(Err(e));
                }
            }
            return Some(Ok(self.class_obj(c) as u64));
        }
        let c = self.class_of(a.first().copied().unwrap_or(0))?;
        let name = self.classes[c].name.clone();
        let r: R<u64> = match key {
            "getNameNative()Ljava/lang/String;" | "getName()Ljava/lang/String;" => Ok(self.str_ref(&java_name(&name)) as u64),
            "getSimpleName()Ljava/lang/String;" => Ok(self.str_ref(&simple_name(&name)) as u64),
            "isArray()Z" => Ok(name.starts_with('[') as u64),
            "isPrimitive()Z" => Ok(is_prim_desc(&name) as u64),
            "isInterface()Z" => Ok((self.classes[c].access & 0x200 != 0) as u64),
            "getModifiers()I" => Ok(self.classes[c].access as u64),
            "desiredAssertionStatus()Z" => Ok(0),
            "getSuperclass()Ljava/lang/Class;" => {
                let acc = self.classes[c].access;
                match self.classes[c].sup {
                    Some(s) if acc & 0x200 == 0 && !is_prim_desc(&name) => Ok(self.class_obj(s) as u64),
                    _ => Ok(0),
                }
            }
            "getComponentType()Ljava/lang/Class;" => {
                if let Some(comp) = name.strip_prefix('[') {
                    let cc = self.load(comp);
                    Ok(self.class_obj(cc) as u64)
                } else {
                    Ok(0)
                }
            }
            "isInstance(Ljava/lang/Object;)Z" => {
                if a[1] == 0 {
                    return Some(Ok(0));
                }
                let oc = self.heap[a[1] as usize].class;
                Ok(self.assignable(oc, c) as u64)
            }
            "isAssignableFrom(Ljava/lang/Class;)Z" => match self.class_of(a[1]) {
                Some(o) => Ok(self.assignable(o, c) as u64),
                None => Err(self.npe()),
            },
            _ => return None,
        };
        Some(r)
    }

    fn arraycopy(&mut self, a: &[u32]) -> R<u64> {
        let (s, sp, d, dp, n) = (a[0], a[1] as i32, a[2], a[3] as i32, a[4] as i32);
        if s == 0 || d == 0 {
            return Err(self.npe());
        }
        let (sc, dc) = (self.heap[s as usize].class, self.heap[d as usize].class);
        let (sn, dn) = (self.classes[sc].name.clone(), self.classes[dc].name.clone());
        if !matches!(self.heap[s as usize].kind, Kind::Array(_)) {
            return Err(self.throw_msg("Ljava/lang/ArrayStoreException;", &format!("source of type {} is not an array", java_name(&sn))));
        }
        if !matches!(self.heap[d as usize].kind, Kind::Array(_)) {
            return Err(self.throw_msg("Ljava/lang/ArrayStoreException;", &format!("destination of type {} is not an array", java_name(&dn))));
        }
        let (sp_prim, dp_prim) = (is_prim_desc(&sn[1..]), is_prim_desc(&dn[1..]));
        if (sp_prim || dp_prim) && sn != dn {
            return Err(self.throw_msg("Ljava/lang/ArrayStoreException;", &format!("arraycopy: type mismatch: can not copy {} into {}", java_name(&sn), java_name(&dn))));
        }
        let (sl, dl) = (self.arr_len(s) as i64, self.arr_len(d) as i64);
        if sp < 0 || dp < 0 || n < 0 || sp as i64 + n as i64 > sl || dp as i64 + n as i64 > dl {
            let m = format!("src.length={} srcPos={} dst.length={} dstPos={} length={}", sl, sp, dl, dp, n);
            return Err(self.throw_msg("Ljava/lang/ArrayIndexOutOfBoundsException;", &m));
        }
        let (sp, dp, n) = (sp as usize, dp as usize, n as usize);
        let part: Vec<u64> = match &self.heap[s as usize].kind {
            Kind::Array(v) => v[sp..sp + n].to_vec(),
            _ => vec![],
        };
        if !sp_prim && sn != dn {
            let comp = self.load(&dn[1..]);
            for (i, &e) in part.iter().enumerate() {
                if e != 0 {
                    let ec = self.heap[e as usize].class;
                    if !self.assignable(ec, comp) {
                        let m = format!("source[{}] of type {} cannot be stored in destination array of type {}", sp + i, java_name(&self.classes[ec].name.clone()), java_name(&dn));
                        return Err(self.throw_msg("Ljava/lang/ArrayStoreException;", &m));
                    }
                }
            }
        }
        if let Kind::Array(v) = &mut self.heap[d as usize].kind {
            v[dp..dp + n].copy_from_slice(&part);
        }
        Ok(0)
    }

    pub(crate) fn system_init(&mut self, c: usize) {
        let nl = self.str_ref("\n") as u64;
        if let Some(&i) = self.classes[c].static_idx.get("lineSeparator") {
            self.classes[c].statics[i] = nl;
        }
    }

    fn system_property(name: &str) -> Option<&'static str> {
        Some(match name {
            "line.separator" => "\n",
            "file.separator" => "/",
            "path.separator" => ":",
            "file.encoding" | "sun.jnu.encoding" | "sun.stdout.encoding" | "sun.stderr.encoding" => "UTF-8",
            "java.vendor" | "java.vm.vendor" => "The Android Project",
            "java.vm.name" => "Dalvik",
            "java.specification.version" => "0.9",
            "java.version" => "0",
            "os.name" => "Linux",
            "os.arch" => "x86_64",
            "user.language" => "en",
            "user.region" => "US",
            "java.io.tmpdir" => "/tmp",
            "user.dir" => "/",
            _ => return None,
        })
    }

    fn system_native(&mut self, key: &str, a: &[u32]) -> Option<R<u64>> {
        let r: R<u64> = match key {
            "arraycopy(Ljava/lang/Object;ILjava/lang/Object;II)V" => self.arraycopy(a),
            _ if key.starts_with("arraycopy") && key.contains("Unchecked(") => self.arraycopy(a),
            "currentTimeMillis()J" => Ok(SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)),
            "nanoTime()J" => Ok(START.get_or_init(Instant::now).elapsed().as_nanos() as u64 + 1_000_000_000),
            "setIn0(Ljava/io/InputStream;)V" | "setOut0(Ljava/io/PrintStream;)V" | "setErr0(Ljava/io/PrintStream;)V" => Ok(0),
            "identityHashCode(Ljava/lang/Object;)I" => Ok(self.identity_hash(a[0]) as u64),
            "lineSeparator()Ljava/lang/String;" => Ok(self.str_ref("\n") as u64),
            "getProperty(Ljava/lang/String;)Ljava/lang/String;" | "getProperty(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;" => {
                if a[0] == 0 {
                    return Some(Err(self.throw_msg("Ljava/lang/NullPointerException;", "key == null")));
                }
                let k = self.string_value(a[0]);
                match Self::system_property(&k) {
                    Some(v) => Ok(self.str_ref(v) as u64),
                    None => Ok(a.get(1).copied().unwrap_or(0) as u64),
                }
            }
            "getenv(Ljava/lang/String;)Ljava/lang/String;" => Ok(0),
            "specialProperties()[Ljava/lang/String;" => Ok(self.new_array("[Ljava/lang/String;", vec![]) as u64),
            _ => return None,
        };
        Some(r)
    }
}

fn math_native(key: &str, a: &[u32]) -> Option<R<u64>> {
    let (n, sig) = key.split_once('(')?;
    let v = match (n, sig) {
        ("sin", "D)D") => dbl(a, 0).sin(),
        ("cos", "D)D") => dbl(a, 0).cos(),
        ("tan", "D)D") => dbl(a, 0).tan(),
        ("asin", "D)D") => dbl(a, 0).asin(),
        ("acos", "D)D") => dbl(a, 0).acos(),
        ("atan", "D)D") => dbl(a, 0).atan(),
        ("exp", "D)D") => dbl(a, 0).exp(),
        ("log", "D)D") => dbl(a, 0).ln(),
        ("log10", "D)D") => dbl(a, 0).log10(),
        ("sqrt", "D)D") => dbl(a, 0).sqrt(),
        ("cbrt", "D)D") => dbl(a, 0).cbrt(),
        ("ceil", "D)D") => dbl(a, 0).ceil(),
        ("floor", "D)D") => dbl(a, 0).floor(),
        ("rint", "D)D") => dbl(a, 0).round_ties_even(),
        ("sinh", "D)D") => dbl(a, 0).sinh(),
        ("cosh", "D)D") => dbl(a, 0).cosh(),
        ("tanh", "D)D") => dbl(a, 0).tanh(),
        ("expm1", "D)D") => dbl(a, 0).exp_m1(),
        ("log1p", "D)D") => dbl(a, 0).ln_1p(),
        ("atan2", "DD)D") => dbl(a, 0).atan2(dbl(a, 2)),
        ("pow", "DD)D") => java_pow(dbl(a, 0), dbl(a, 2)),
        ("hypot", "DD)D") => dbl(a, 0).hypot(dbl(a, 2)),
        ("IEEEremainder", "DD)D") => ieee_remainder(dbl(a, 0), dbl(a, 2)),
        _ => return None,
    };
    Some(Ok(v.to_bits()))
}

fn bits_native(key: &str, a: &[u32]) -> Option<R<u64>> {
    match key {
        "floatToRawIntBits(F)I" | "intBitsToFloat(I)F" => Some(Ok(a[0] as u64)),
        "doubleToRawLongBits(D)J" | "longBitsToDouble(J)D" => Some(Ok((a[0] as u64) | ((a[1] as u64) << 32))),
        _ => None,
    }
}

fn char_type(c: char) -> i32 {
    if c.is_uppercase() {
        1
    } else if c.is_lowercase() {
        2
    } else if c.is_alphabetic() {
        5
    } else if c.is_numeric() {
        if c.to_digit(10).is_some() || c.is_ascii_digit() { 9 } else { 11 }
    } else if c == ' ' || c == '\u{a0}' || c == '\u{3000}' {
        12
    } else if c == '\u{2028}' {
        13
    } else if c == '\u{2029}' {
        14
    } else if c.is_control() {
        15
    } else if c.is_whitespace() {
        12
    } else {
        match c {
            '_' => 23,
            '-' => 20,
            '(' | '[' | '{' => 21,
            ')' | ']' | '}' => 22,
            '+' | '<' | '=' | '>' | '|' | '~' => 25,
            '$' => 26,
            '^' | '`' => 27,
            _ => 24,
        }
    }
}

fn char_native(key: &str, a: &[u32]) -> Option<R<u64>> {
    let (n, sig) = key.split_once('(')?;
    if !n.ends_with("Impl") {
        return None;
    }
    let cp = *a.first()?;
    let ch = char::from_u32(cp);
    let b = |v: bool| Some(Ok(v as u64));
    let case = |up: bool| -> Option<R<u64>> {
        match ch {
            None => Some(Ok(cp as u64)),
            Some(c) => {
                let mut it: Vec<char> = if up { c.to_uppercase().collect() } else { c.to_lowercase().collect() };
                if it.len() != 1 {
                    it = vec![c];
                }
                Some(Ok(it[0] as u32 as u64))
            }
        }
    };
    match (n, sig) {
        ("isLowerCaseImpl", "I)Z") => b(ch.map_or(false, |c| c.is_lowercase())),
        ("isUpperCaseImpl", "I)Z") => b(ch.map_or(false, |c| c.is_uppercase())),
        ("isTitleCaseImpl", "I)Z") => b(matches!(cp, 0x1C5 | 0x1C8 | 0x1CB | 0x1F2)),
        ("isDigitImpl", "I)Z") => b(ch.map_or(false, |c| char_type(c) == 9)),
        ("isDefinedImpl", "I)Z") => b(ch.is_some()),
        ("isLetterImpl", "I)Z") => b(ch.map_or(false, |c| c.is_alphabetic() && !c.is_numeric())),
        ("isLetterOrDigitImpl", "I)Z") => b(ch.map_or(false, |c| c.is_alphabetic() || char_type(c) == 9)),
        ("isAlphabeticImpl", "I)Z") => b(ch.map_or(false, |c| c.is_alphabetic())),
        ("isIdeographicImpl", "I)Z") => b((0x4E00..=0x9FFF).contains(&cp) || (0x3400..=0x4DBF).contains(&cp)),
        ("isUnicodeIdentifierStartImpl", "I)Z") => b(ch.map_or(false, |c| c.is_alphabetic())),
        ("isUnicodeIdentifierPartImpl", "I)Z") => b(ch.map_or(false, |c| c.is_alphanumeric() || c == '_')),
        ("isIdentifierIgnorableImpl", "I)Z") => b(cp <= 8 || (0xE..=0x1B).contains(&cp) || (0x7F..=0x9F).contains(&cp)),
        ("toLowerCaseImpl", "I)I") => case(false),
        ("toUpperCaseImpl", "I)I") | ("toTitleCaseImpl", "I)I") => case(true),
        ("digitImpl", "II)I") => {
            let r = a[1];
            let d = ch.and_then(|c| if char_type(c) == 9 { c.to_digit(10) } else { c.to_digit(36) });
            let d = match ch {
                Some(c) if c.is_ascii_alphanumeric() => c.to_digit(36),
                _ => d,
            };
            Some(Ok(match d {
                Some(v) if (2..=36).contains(&r) && v < r => v as u64,
                _ => u32::MAX as u64,
            }))
        }
        ("getNumericValueImpl", "I)I") => Some(Ok(match ch {
            Some(c) if c.is_ascii_alphanumeric() => c.to_digit(36).unwrap_or(u32::MAX),
            Some(c) => c.to_digit(10).unwrap_or(u32::MAX),
            None => u32::MAX,
        } as u64)),
        ("isSpaceCharImpl", "I)Z") => b(ch.map_or(false, |c| matches!(char_type(c), 12 | 13 | 14))),
        ("isWhitespaceImpl", "I)Z") => b(ch.map_or(false, |c| (c.is_whitespace() && !matches!(c, '\u{a0}' | '\u{2007}' | '\u{202f}')) || ('\u{1c}'..='\u{1f}').contains(&c))),
        ("getTypeImpl", "I)I") => Some(Ok(ch.map_or(0, char_type) as u64)),
        ("getDirectionalityImpl", "I)B") => Some(Ok(0)),
        ("isMirroredImpl", "I)Z") => b(matches!(ch, Some('(' | ')' | '<' | '>' | '[' | ']' | '{' | '}'))),
        _ => None,
    }
}

impl Runtime {
    fn reference_native(&mut self, key: &str, a: &[u32]) -> Option<R<u64>> {
        let this = *a.first()?;
        if this == 0 {
            return None;
        }
        let c = self.heap[this as usize].class;
        let slot = *self.classes[c].inst_idx.get("referent")?;
        let f = &mut self.heap[this as usize].fields;
        if f.len() <= slot {
            f.resize(slot + 1, 0);
        }
        match key {
            "getReferent()Ljava/lang/Object;" => Some(Ok(f[slot])),
            "refersTo0(Ljava/lang/Object;)Z" => Some(Ok((f[slot] as u32 == a.get(1).copied().unwrap_or(0)) as u64)),
            "clearReferent()V" => {
                f[slot] = 0;
                Some(Ok(0))
            }
            _ => None,
        }
    }
}

fn vmruntime_native(key: &str, a: &[u32]) -> Option<R<u64>> {
    match key {
        "getSdkVersionNative(I)I" => Some(Ok(a.first().copied().unwrap_or(0) as u64)),
        "is64Bit()Z" => Some(Ok(1)),
        "isCheckJniEnabled()Z" | "isNativeDebuggable()Z" | "isJavaDebuggable()Z" => Some(Ok(0)),
        "getTargetHeapUtilization()F" => Some(Ok(0.75f32.to_bits() as u64)),
        "setTargetSdkVersionNative(I)V" | "setDisabledCompatChangesNative([J)V" | "nativeSetTargetHeapUtilization(F)V" | "clearGrowthLimit()V" | "clampGrowthLimit()V" => Some(Ok(0)),
        _ => None,
    }
}
