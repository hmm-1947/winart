#![allow(dead_code, non_upper_case_globals)]
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

#[link(name = "kernel32")]
extern "system" {
    fn GetProcessHeap() -> *mut c_void;
    fn HeapAlloc(h: *mut c_void, flags: u32, n: usize) -> *mut u8;
    fn HeapReAlloc(h: *mut c_void, flags: u32, p: *mut u8, n: usize) -> *mut u8;
    fn HeapFree(h: *mut c_void, flags: u32, p: *mut u8) -> i32;
    fn HeapSize(h: *mut c_void, flags: u32, p: *const u8) -> usize;
    fn VirtualAlloc(addr: *mut u8, size: usize, ty: u32, prot: u32) -> *mut u8;
    fn VirtualFree(addr: *mut u8, size: usize, ty: u32) -> i32;
    fn GetCurrentThreadId() -> u32;
    fn GetCurrentProcessId() -> u32;
}

#[repr(C)]
pub struct VaList {
    gp_offset: u32,
    fp_offset: u32,
    overflow: *mut u8,
    reg_save: *mut u8,
}

impl VaList {
    pub unsafe fn gp(&mut self) -> u64 {
        if self.gp_offset < 48 {
            let v = *(self.reg_save.add(self.gp_offset as usize) as *const u64);
            self.gp_offset += 8;
            v
        } else {
            let v = *(self.overflow as *const u64);
            self.overflow = self.overflow.add(8);
            v
        }
    }

    pub unsafe fn fp(&mut self) -> f64 {
        if self.fp_offset < 176 {
            let v = *(self.reg_save.add(self.fp_offset as usize) as *const f64);
            self.fp_offset += 16;
            v
        } else {
            let v = *(self.overflow as *const f64);
            self.overflow = self.overflow.add(8);
            v
        }
    }
}

pub unsafe fn cstr<'a>(p: *const u8) -> &'a [u8] {
    if p.is_null() {
        return &[];
    }
    let mut n = 0usize;
    while *p.add(n) != 0 {
        n += 1;
    }
    std::slice::from_raw_parts(p, n)
}

pub unsafe fn cstring(p: *const u8) -> String {
    String::from_utf8_lossy(cstr(p)).into_owned()
}

static ALIGNED: Mutex<Option<HashMap<usize, usize>>> = Mutex::new(None);
static ANY_ALIGNED: AtomicBool = AtomicBool::new(false);

pub unsafe extern "sysv64" fn malloc(n: usize) -> *mut u8 {
    HeapAlloc(GetProcessHeap(), 0, n.max(1))
}

pub unsafe extern "sysv64" fn calloc(a: usize, b: usize) -> *mut u8 {
    HeapAlloc(GetProcessHeap(), 8, a.saturating_mul(b).max(1))
}

unsafe extern "sysv64" fn realloc(p: *mut u8, n: usize) -> *mut u8 {
    if p.is_null() {
        return malloc(n);
    }
    HeapReAlloc(GetProcessHeap(), 0, p, n.max(1))
}

pub unsafe extern "sysv64" fn free(p: *mut u8) {
    if p.is_null() {
        return;
    }
    if ANY_ALIGNED.load(Ordering::Relaxed) {
        let mut g = ALIGNED.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(m) = g.as_mut() {
            if let Some(raw) = m.remove(&(p as usize)) {
                HeapFree(GetProcessHeap(), 0, raw as *mut u8);
                return;
            }
        }
    }
    HeapFree(GetProcessHeap(), 0, p);
}

unsafe extern "sysv64" fn memalign(align: usize, n: usize) -> *mut u8 {
    let a = align.max(16).next_power_of_two();
    let raw = HeapAlloc(GetProcessHeap(), 0, n + a);
    if raw.is_null() {
        return raw;
    }
    let al = (raw as usize + a - 1) & !(a - 1);
    if al != raw as usize {
        let mut g = ALIGNED.lock().unwrap_or_else(|e| e.into_inner());
        g.get_or_insert_with(HashMap::new).insert(al, raw as usize);
        ANY_ALIGNED.store(true, Ordering::Relaxed);
    }
    al as *mut u8
}

unsafe extern "sysv64" fn posix_memalign(out: *mut *mut u8, align: usize, n: usize) -> i32 {
    let p = memalign(align, n);
    if p.is_null() {
        return 12;
    }
    *out = p;
    0
}

unsafe extern "sysv64" fn malloc_usable_size(p: *const u8) -> usize {
    if p.is_null() {
        return 0;
    }
    HeapSize(GetProcessHeap(), 0, p)
}

unsafe extern "sysv64" fn memcpy(d: *mut u8, s: *const u8, n: usize) -> *mut u8 {
    std::ptr::copy(s, d, n);
    d
}

unsafe extern "sysv64" fn memset(d: *mut u8, c: i32, n: usize) -> *mut u8 {
    std::ptr::write_bytes(d, c as u8, n);
    d
}

unsafe extern "sysv64" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    for i in 0..n {
        let (x, y) = (*a.add(i), *b.add(i));
        if x != y {
            return x as i32 - y as i32;
        }
    }
    0
}

unsafe extern "sysv64" fn memchr(s: *const u8, c: i32, n: usize) -> *mut u8 {
    for i in 0..n {
        if *s.add(i) == c as u8 {
            return s.add(i) as *mut u8;
        }
    }
    std::ptr::null_mut()
}

unsafe extern "sysv64" fn memrchr(s: *const u8, c: i32, n: usize) -> *mut u8 {
    let mut i = n;
    while i > 0 {
        i -= 1;
        if *s.add(i) == c as u8 {
            return s.add(i) as *mut u8;
        }
    }
    std::ptr::null_mut()
}

unsafe extern "sysv64" fn strlen(s: *const u8) -> usize {
    cstr(s).len()
}

unsafe extern "sysv64" fn strnlen(s: *const u8, n: usize) -> usize {
    let mut i = 0;
    while i < n && *s.add(i) != 0 {
        i += 1;
    }
    i
}

unsafe extern "sysv64" fn strcmp(a: *const u8, b: *const u8) -> i32 {
    let mut i = 0;
    loop {
        let (x, y) = (*a.add(i), *b.add(i));
        if x != y || x == 0 {
            return x as i32 - y as i32;
        }
        i += 1;
    }
}

unsafe extern "sysv64" fn strncmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    for i in 0..n {
        let (x, y) = (*a.add(i), *b.add(i));
        if x != y || x == 0 {
            return x as i32 - y as i32;
        }
    }
    0
}

unsafe extern "sysv64" fn strcasecmp(a: *const u8, b: *const u8) -> i32 {
    let mut i = 0;
    loop {
        let (x, y) = ((*a.add(i)).to_ascii_lowercase(), (*b.add(i)).to_ascii_lowercase());
        if x != y || x == 0 {
            return x as i32 - y as i32;
        }
        i += 1;
    }
}

unsafe extern "sysv64" fn strncasecmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    for i in 0..n {
        let (x, y) = ((*a.add(i)).to_ascii_lowercase(), (*b.add(i)).to_ascii_lowercase());
        if x != y || x == 0 {
            return x as i32 - y as i32;
        }
    }
    0
}

unsafe extern "sysv64" fn strcpy(d: *mut u8, s: *const u8) -> *mut u8 {
    let n = cstr(s).len();
    std::ptr::copy(s, d, n + 1);
    d
}

unsafe extern "sysv64" fn strncpy(d: *mut u8, s: *const u8, n: usize) -> *mut u8 {
    let l = strnlen(s, n);
    std::ptr::copy(s, d, l);
    if l < n {
        std::ptr::write_bytes(d.add(l), 0, n - l);
    }
    d
}

unsafe extern "sysv64" fn strcat(d: *mut u8, s: *const u8) -> *mut u8 {
    let dl = cstr(d).len();
    let n = cstr(s).len();
    std::ptr::copy(s, d.add(dl), n + 1);
    d
}

unsafe extern "sysv64" fn strncat(d: *mut u8, s: *const u8, n: usize) -> *mut u8 {
    let dl = cstr(d).len();
    let l = strnlen(s, n);
    std::ptr::copy(s, d.add(dl), l);
    *d.add(dl + l) = 0;
    d
}

unsafe extern "sysv64" fn strchr(s: *const u8, c: i32) -> *mut u8 {
    let mut i = 0;
    loop {
        let x = *s.add(i);
        if x == c as u8 {
            return s.add(i) as *mut u8;
        }
        if x == 0 {
            return std::ptr::null_mut();
        }
        i += 1;
    }
}

unsafe extern "sysv64" fn strrchr(s: *const u8, c: i32) -> *mut u8 {
    let n = cstr(s).len();
    let mut i = n + 1;
    while i > 0 {
        i -= 1;
        if *s.add(i) == c as u8 {
            return s.add(i) as *mut u8;
        }
    }
    std::ptr::null_mut()
}

unsafe extern "sysv64" fn strstr(h: *const u8, n: *const u8) -> *mut u8 {
    let hs = cstr(h);
    let ns = cstr(n);
    if ns.is_empty() {
        return h as *mut u8;
    }
    if ns.len() > hs.len() {
        return std::ptr::null_mut();
    }
    for i in 0..=(hs.len() - ns.len()) {
        if &hs[i..i + ns.len()] == ns {
            return h.add(i) as *mut u8;
        }
    }
    std::ptr::null_mut()
}

unsafe extern "sysv64" fn strspn(s: *const u8, set: *const u8) -> usize {
    let st = cstr(set);
    let mut i = 0;
    while *s.add(i) != 0 && st.contains(&*s.add(i)) {
        i += 1;
    }
    i
}

unsafe extern "sysv64" fn strcspn(s: *const u8, set: *const u8) -> usize {
    let st = cstr(set);
    let mut i = 0;
    while *s.add(i) != 0 && !st.contains(&*s.add(i)) {
        i += 1;
    }
    i
}

unsafe extern "sysv64" fn strpbrk(s: *const u8, set: *const u8) -> *mut u8 {
    let i = strcspn(s, set);
    if *s.add(i) == 0 {
        std::ptr::null_mut()
    } else {
        s.add(i) as *mut u8
    }
}

unsafe extern "sysv64" fn strdup(s: *const u8) -> *mut u8 {
    let n = cstr(s).len();
    let p = malloc(n + 1);
    std::ptr::copy(s, p, n + 1);
    p
}

unsafe extern "sysv64" fn strndup(s: *const u8, n: usize) -> *mut u8 {
    let l = strnlen(s, n);
    let p = malloc(l + 1);
    std::ptr::copy(s, p, l);
    *p.add(l) = 0;
    p
}

thread_local! {
    static STRTOK: Cell<usize> = const { Cell::new(0) };
}

unsafe extern "sysv64" fn strtok(s: *mut u8, delim: *const u8) -> *mut u8 {
    let mut p = if s.is_null() { STRTOK.with(|c| c.get()) as *mut u8 } else { s };
    if p.is_null() {
        return p;
    }
    p = p.add(strspn(p, delim));
    if *p == 0 {
        STRTOK.with(|c| c.set(0));
        return std::ptr::null_mut();
    }
    let e = p.add(strcspn(p, delim));
    if *e == 0 {
        STRTOK.with(|c| c.set(0));
    } else {
        *e = 0;
        STRTOK.with(|c| c.set(e as usize + 1));
    }
    p
}

unsafe extern "sysv64" fn strlen_chk(s: *const u8, _size: usize) -> usize {
    cstr(s).len()
}

unsafe extern "sysv64" fn memcpy_chk(d: *mut u8, s: *const u8, n: usize, _ds: usize) -> *mut u8 {
    memcpy(d, s, n)
}

unsafe extern "sysv64" fn memset_chk(d: *mut u8, c: i32, n: usize, _ds: usize) -> *mut u8 {
    memset(d, c, n)
}

unsafe extern "sysv64" fn strcpy_chk(d: *mut u8, s: *const u8, _ds: usize) -> *mut u8 {
    strcpy(d, s)
}

unsafe extern "sysv64" fn strncpy_chk(d: *mut u8, s: *const u8, n: usize, _ds: usize) -> *mut u8 {
    strncpy(d, s, n)
}

unsafe extern "sysv64" fn strcat_chk(d: *mut u8, s: *const u8, _ds: usize) -> *mut u8 {
    strcat(d, s)
}

unsafe extern "sysv64" fn strncat_chk(d: *mut u8, s: *const u8, n: usize, _ds: usize) -> *mut u8 {
    strncat(d, s, n)
}

unsafe extern "sysv64" fn toupper(c: i32) -> i32 {
    if (97..=122).contains(&c) {
        c - 32
    } else {
        c
    }
}

unsafe extern "sysv64" fn tolower(c: i32) -> i32 {
    if (65..=90).contains(&c) {
        c + 32
    } else {
        c
    }
}

unsafe extern "sysv64" fn isalpha(c: i32) -> i32 {
    (0..256).contains(&c) as i32 & (c as u8).is_ascii_alphabetic() as i32
}

unsafe extern "sysv64" fn isdigit(c: i32) -> i32 {
    (0x30..=0x39).contains(&c) as i32
}

unsafe extern "sysv64" fn isalnum(c: i32) -> i32 {
    (0..256).contains(&c) as i32 & (c as u8).is_ascii_alphanumeric() as i32
}

unsafe extern "sysv64" fn isspace(c: i32) -> i32 {
    matches!(c, 32 | 9..=13) as i32
}

unsafe extern "sysv64" fn isupper(c: i32) -> i32 {
    (65..=90).contains(&c) as i32
}

unsafe extern "sysv64" fn islower(c: i32) -> i32 {
    (97..=122).contains(&c) as i32
}

unsafe extern "sysv64" fn isxdigit(c: i32) -> i32 {
    (0..256).contains(&c) as i32 & (c as u8).is_ascii_hexdigit() as i32
}

unsafe extern "sysv64" fn ispunct(c: i32) -> i32 {
    (0..256).contains(&c) as i32 & (c as u8).is_ascii_punctuation() as i32
}

unsafe extern "sysv64" fn isprint(c: i32) -> i32 {
    (32..127).contains(&c) as i32
}

struct Spec {
    left: bool,
    plus: bool,
    space: bool,
    alt: bool,
    zero: bool,
    width: usize,
    prec: Option<usize>,
}

fn emit(out: &mut Vec<u8>, prefix: &[u8], body: &[u8], sp: &Spec, zero_ok: bool) {
    let total = prefix.len() + body.len();
    if sp.width > total {
        let n = sp.width - total;
        if sp.left {
            out.extend_from_slice(prefix);
            out.extend_from_slice(body);
            out.extend(std::iter::repeat(b' ').take(n));
        } else if sp.zero && zero_ok {
            out.extend_from_slice(prefix);
            out.extend(std::iter::repeat(b'0').take(n));
            out.extend_from_slice(body);
        } else {
            out.extend(std::iter::repeat(b' ').take(n));
            out.extend_from_slice(prefix);
            out.extend_from_slice(body);
        }
    } else {
        out.extend_from_slice(prefix);
        out.extend_from_slice(body);
    }
}

fn sign_prefix(neg: bool, sp: &Spec) -> &'static [u8] {
    if neg {
        b"-"
    } else if sp.plus {
        b"+"
    } else if sp.space {
        b" "
    } else {
        b""
    }
}

fn fmt_f(v: f64, prec: usize, alt: bool) -> String {
    let mut s = format!("{:.*}", prec, v);
    if alt && prec == 0 {
        s.push('.');
    }
    s
}

fn fmt_e(v: f64, prec: usize, upper: bool) -> String {
    let s = format!("{:.*e}", prec, v);
    let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
    let ev: i32 = e.parse().unwrap_or(0);
    format!("{}{}{}{:02}", m, if upper { 'E' } else { 'e' }, if ev < 0 { '-' } else { '+' }, ev.abs())
}

fn strip_zeros(s: &mut String) {
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
}

fn fmt_g(v: f64, prec: usize, alt: bool, upper: bool) -> String {
    let p = if prec == 0 { 1 } else { prec };
    let x: i32 = if v == 0.0 {
        0
    } else {
        let s = format!("{:.*e}", p - 1, v);
        s.split_once('e').and_then(|(_, e)| e.parse().ok()).unwrap_or(0)
    };
    if (p as i32) > x && x >= -4 {
        let mut s = fmt_f(v, (p as i32 - 1 - x) as usize, alt);
        if !alt {
            strip_zeros(&mut s);
        }
        s
    } else {
        let s = fmt_e(v, p - 1, upper);
        if alt {
            return s;
        }
        let epos = s.find(if upper { 'E' } else { 'e' }).unwrap_or(s.len());
        let (m, e) = s.split_at(epos);
        let mut m = m.to_string();
        strip_zeros(&mut m);
        format!("{}{}", m, e)
    }
}

unsafe fn fmt_core(fmt: *const u8, ap: &mut VaList) -> Vec<u8> {
    let f = cstr(fmt);
    let mut out: Vec<u8> = Vec::new();
    let mut i = 0usize;
    while i < f.len() {
        let c = f[i];
        if c != b'%' {
            out.push(c);
            i += 1;
            continue;
        }
        i += 1;
        if i >= f.len() {
            break;
        }
        let mut sp = Spec { left: false, plus: false, space: false, alt: false, zero: false, width: 0, prec: None };
        while i < f.len() {
            match f[i] {
                b'-' => sp.left = true,
                b'+' => sp.plus = true,
                b' ' => sp.space = true,
                b'#' => sp.alt = true,
                b'0' => sp.zero = true,
                _ => break,
            }
            i += 1;
        }
        if i < f.len() && f[i] == b'*' {
            let w = ap.gp() as i32;
            if w < 0 {
                sp.left = true;
                sp.width = (-(w as i64)) as usize;
            } else {
                sp.width = w as usize;
            }
            i += 1;
        } else {
            while i < f.len() && f[i].is_ascii_digit() {
                sp.width = sp.width * 10 + (f[i] - b'0') as usize;
                i += 1;
            }
        }
        if i < f.len() && f[i] == b'.' {
            i += 1;
            if i < f.len() && f[i] == b'*' {
                let v = ap.gp() as i32;
                sp.prec = if v >= 0 { Some(v as usize) } else { None };
                i += 1;
            } else {
                let mut p = 0usize;
                while i < f.len() && f[i].is_ascii_digit() {
                    p = p * 10 + (f[i] - b'0') as usize;
                    i += 1;
                }
                sp.prec = Some(p);
            }
        }
        let mut len = 0u8;
        loop {
            match f.get(i) {
                Some(b'h') => {
                    len = if len == 2 { 1 } else { 2 };
                    i += 1;
                }
                Some(b'l') => {
                    len = if len == 3 { 4 } else { 3 };
                    i += 1;
                }
                Some(b'z') | Some(b'j') | Some(b't') => {
                    len = 5;
                    i += 1;
                }
                Some(b'L') => {
                    len = 6;
                    i += 1;
                }
                _ => break,
            }
        }
        let conv = match f.get(i) {
            Some(&c) => c,
            None => break,
        };
        i += 1;
        match conv {
            b'd' | b'i' => {
                let raw = ap.gp();
                let v: i64 = match len {
                    1 => raw as i8 as i64,
                    2 => raw as i16 as i64,
                    3 | 4 | 5 => raw as i64,
                    _ => raw as i32 as i64,
                };
                let mut digits = v.unsigned_abs().to_string();
                if let Some(p) = sp.prec {
                    if p == 0 && v == 0 {
                        digits.clear();
                    }
                    while digits.len() < p {
                        digits.insert(0, '0');
                    }
                }
                emit(&mut out, sign_prefix(v < 0, &sp), digits.as_bytes(), &sp, sp.prec.is_none());
            }
            b'u' | b'x' | b'X' | b'o' => {
                let raw = ap.gp();
                let v: u64 = match len {
                    1 => raw as u8 as u64,
                    2 => raw as u16 as u64,
                    3 | 4 | 5 => raw,
                    _ => raw as u32 as u64,
                };
                let mut digits = match conv {
                    b'u' => v.to_string(),
                    b'x' => format!("{:x}", v),
                    b'X' => format!("{:X}", v),
                    _ => format!("{:o}", v),
                };
                if let Some(p) = sp.prec {
                    if p == 0 && v == 0 {
                        digits.clear();
                    }
                    while digits.len() < p {
                        digits.insert(0, '0');
                    }
                }
                let mut prefix: &[u8] = b"";
                if sp.alt {
                    if conv == b'x' && v != 0 {
                        prefix = b"0x";
                    } else if conv == b'X' && v != 0 {
                        prefix = b"0X";
                    } else if conv == b'o' && !digits.starts_with('0') {
                        digits.insert(0, '0');
                    }
                }
                emit(&mut out, prefix, digits.as_bytes(), &sp, sp.prec.is_none());
            }
            b'c' => {
                let v = ap.gp() as u8;
                emit(&mut out, b"", &[v], &sp, false);
            }
            b's' => {
                let p = ap.gp() as *const u8;
                let s: &[u8] = if p.is_null() { b"(null)" } else { cstr(p) };
                let s = match sp.prec {
                    Some(n) if n < s.len() => &s[..n],
                    _ => s,
                };
                emit(&mut out, b"", s, &sp, false);
            }
            b'p' => {
                let v = ap.gp();
                if v == 0 {
                    emit(&mut out, b"", b"(nil)", &sp, false);
                } else {
                    emit(&mut out, b"0x", format!("{:x}", v).as_bytes(), &sp, false);
                }
            }
            b'f' | b'F' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A' => {
                let v = ap.fp();
                let neg = v.is_sign_negative() && !v.is_nan();
                let a = v.abs();
                let prec = sp.prec.unwrap_or(6);
                let upper = conv.is_ascii_uppercase();
                let mut body = if a.is_nan() {
                    "nan".to_string()
                } else if a.is_infinite() {
                    "inf".to_string()
                } else {
                    match conv {
                        b'f' | b'F' => fmt_f(a, prec, sp.alt),
                        b'e' | b'E' | b'a' | b'A' => fmt_e(a, prec, upper),
                        _ => fmt_g(a, prec, sp.alt, upper),
                    }
                };
                if upper && !a.is_finite() {
                    body = body.to_uppercase();
                }
                emit(&mut out, sign_prefix(neg, &sp), body.as_bytes(), &sp, a.is_finite());
            }
            b'%' => out.push(b'%'),
            b'n' => {
                ap.gp();
            }
            other => {
                out.push(b'%');
                out.push(other);
            }
        }
    }
    out
}

pub unsafe extern "sysv64" fn vsnprintf(buf: *mut u8, size: usize, fmt: *const u8, ap: *mut VaList) -> i32 {
    let out = fmt_core(fmt, &mut *ap);
    if size > 0 && !buf.is_null() {
        let n = out.len().min(size - 1);
        std::ptr::copy_nonoverlapping(out.as_ptr(), buf, n);
        *buf.add(n) = 0;
    }
    out.len() as i32
}

unsafe extern "sysv64" fn vsprintf(buf: *mut u8, fmt: *const u8, ap: *mut VaList) -> i32 {
    let out = fmt_core(fmt, &mut *ap);
    std::ptr::copy_nonoverlapping(out.as_ptr(), buf, out.len());
    *buf.add(out.len()) = 0;
    out.len() as i32
}

unsafe extern "sysv64" fn vsnprintf_chk(buf: *mut u8, size: usize, _flag: i32, _bs: usize, fmt: *const u8, ap: *mut VaList) -> i32 {
    vsnprintf(buf, size, fmt, ap)
}

unsafe extern "sysv64" fn vsprintf_chk(buf: *mut u8, _flag: i32, _bs: usize, fmt: *const u8, ap: *mut VaList) -> i32 {
    vsprintf(buf, fmt, ap)
}

unsafe extern "sysv64" fn vprintf(fmt: *const u8, ap: *mut VaList) -> i32 {
    use std::io::Write;
    let out = fmt_core(fmt, &mut *ap);
    let so = std::io::stdout();
    let mut l = so.lock();
    let _ = l.write_all(&out);
    let _ = l.flush();
    out.len() as i32
}

unsafe extern "sysv64" fn puts(s: *const u8) -> i32 {
    use std::io::Write;
    let so = std::io::stdout();
    let mut l = so.lock();
    let _ = l.write_all(cstr(s));
    let _ = l.write_all(b"\n");
    let _ = l.flush();
    1
}

unsafe extern "sysv64" fn putchar(c: i32) -> i32 {
    use std::io::Write;
    let _ = std::io::stdout().write_all(&[c as u8]);
    c
}

unsafe extern "sysv64" fn fflush(_f: *mut u8) -> i32 {
    0
}

pub static NATIVE_LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn log_line(prio: i32, tag: &[u8], msg: &[u8]) {
    let p = match prio {
        2 => 'V',
        3 => 'D',
        4 => 'I',
        5 => 'W',
        6 => 'E',
        7 => 'F',
        _ => '?',
    };
    let line = format!("{}/{}: {}", p, String::from_utf8_lossy(tag), String::from_utf8_lossy(msg));
    eprintln!("[native] {}", line);
    let mut g = NATIVE_LOG.lock().unwrap_or_else(|e| e.into_inner());
    if g.len() > 5000 {
        g.drain(..2500);
    }
    g.push(line);
}

pub fn native_log_contains(s: &str) -> bool {
    NATIVE_LOG.lock().unwrap_or_else(|e| e.into_inner()).iter().any(|l| l.contains(s))
}

unsafe extern "sysv64" fn android_log_vprint(prio: i32, tag: *const u8, fmt: *const u8, ap: *mut VaList) -> i32 {
    let out = fmt_core(fmt, &mut *ap);
    log_line(prio, cstr(tag), &out);
    out.len() as i32
}

unsafe extern "sysv64" fn android_log_write(prio: i32, tag: *const u8, text: *const u8) -> i32 {
    log_line(prio, cstr(tag), cstr(text));
    1
}

unsafe extern "sysv64" fn android_log_buf_write(_buf: i32, prio: i32, tag: *const u8, text: *const u8) -> i32 {
    log_line(prio, cstr(tag), cstr(text));
    1
}

unsafe extern "sysv64" fn android_log_assert(cond: *const u8, tag: *const u8, fmt: *const u8) -> ! {
    log_line(7, cstr(tag), cstr(fmt));
    eprintln!("winart: native assertion failed: {}", cstring(cond));
    std::process::exit(134);
}

unsafe extern "sysv64" fn android_log_is_loggable(_prio: i32, _tag: *const u8, _def: i32) -> i32 {
    1
}

unsafe fn scan_int(s: *const u8, end: *mut *mut u8, mut base: i32) -> (bool, u64, bool) {
    let mut p = s;
    while isspace(*p as i32) != 0 {
        p = p.add(1);
    }
    let mut neg = false;
    if *p == b'-' {
        neg = true;
        p = p.add(1);
    } else if *p == b'+' {
        p = p.add(1);
    }
    if (base == 0 || base == 16) && *p == b'0' && (*p.add(1) == b'x' || *p.add(1) == b'X') && (*p.add(2)).is_ascii_hexdigit() {
        p = p.add(2);
        base = 16;
    } else if base == 0 {
        base = if *p == b'0' { 8 } else { 10 };
    }
    let start = p;
    let mut v: u64 = 0;
    let mut ovf = false;
    loop {
        let c = *p;
        let d: i32 = if c.is_ascii_digit() {
            (c - b'0') as i32
        } else if c.is_ascii_lowercase() {
            (c - b'a') as i32 + 10
        } else if c.is_ascii_uppercase() {
            (c - b'A') as i32 + 10
        } else {
            99
        };
        if d >= base {
            break;
        }
        match v.checked_mul(base as u64).and_then(|x| x.checked_add(d as u64)) {
            Some(x) => v = x,
            None => ovf = true,
        }
        p = p.add(1);
    }
    if p == start {
        if !end.is_null() {
            *end = s as *mut u8;
        }
        return (false, 0, false);
    }
    if !end.is_null() {
        *end = p as *mut u8;
    }
    (neg, v, ovf)
}

unsafe extern "sysv64" fn strtoll(s: *const u8, end: *mut *mut u8, base: i32) -> i64 {
    let (neg, v, ovf) = scan_int(s, end, base);
    if neg {
        if ovf || v > (1u64 << 63) {
            i64::MIN
        } else {
            (v as i64).wrapping_neg()
        }
    } else if ovf || v > i64::MAX as u64 {
        i64::MAX
    } else {
        v as i64
    }
}

unsafe extern "sysv64" fn strtoull(s: *const u8, end: *mut *mut u8, base: i32) -> u64 {
    let (neg, v, ovf) = scan_int(s, end, base);
    if ovf {
        u64::MAX
    } else if neg {
        v.wrapping_neg()
    } else {
        v
    }
}

unsafe extern "sysv64" fn atoi(s: *const u8) -> i32 {
    strtoll(s, std::ptr::null_mut(), 10) as i32
}

unsafe extern "sysv64" fn atol(s: *const u8) -> i64 {
    strtoll(s, std::ptr::null_mut(), 10)
}

unsafe extern "sysv64" fn strtod(s: *const u8, end: *mut *mut u8) -> f64 {
    let b = cstr(s);
    let mut i = 0;
    while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) {
        i += 1;
    }
    let st = i;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    let rest = String::from_utf8_lossy(&b[i..]).to_ascii_lowercase();
    let mut consumed = 0usize;
    let mut special: Option<f64> = None;
    if rest.starts_with("infinity") {
        consumed = 8;
        special = Some(f64::INFINITY);
    } else if rest.starts_with("inf") {
        consumed = 3;
        special = Some(f64::INFINITY);
    } else if rest.starts_with("nan") {
        consumed = 3;
        special = Some(f64::NAN);
    }
    let neg = b.get(st) == Some(&b'-');
    if let Some(v) = special {
        if !end.is_null() {
            *end = s.add(i + consumed) as *mut u8;
        }
        return if neg { -v } else { v };
    }
    let mut j = i;
    let mut digits = 0;
    while j < b.len() && b[j].is_ascii_digit() {
        j += 1;
        digits += 1;
    }
    if j < b.len() && b[j] == b'.' {
        j += 1;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        if !end.is_null() {
            *end = s as *mut u8;
        }
        return 0.0;
    }
    if j < b.len() && (b[j] == b'e' || b[j] == b'E') {
        let mut k = j + 1;
        if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
            k += 1;
        }
        let ds = k;
        while k < b.len() && b[k].is_ascii_digit() {
            k += 1;
        }
        if k > ds {
            j = k;
        }
    }
    let txt = String::from_utf8_lossy(&b[i..j]).into_owned();
    let v: f64 = txt.parse().unwrap_or(0.0);
    if !end.is_null() {
        *end = s.add(j) as *mut u8;
    }
    if neg {
        -v
    } else {
        v
    }
}

unsafe extern "sysv64" fn strtof(s: *const u8, end: *mut *mut u8) -> f32 {
    strtod(s, end) as f32
}

unsafe extern "sysv64" fn atof(s: *const u8) -> f64 {
    strtod(s, std::ptr::null_mut())
}

unsafe extern "sysv64" fn abs_i(x: i32) -> i32 {
    x.wrapping_abs()
}

unsafe extern "sysv64" fn labs(x: i64) -> i64 {
    x.wrapping_abs()
}

unsafe extern "sysv64" fn qsort(base: *mut u8, n: usize, sz: usize, cmp: unsafe extern "sysv64" fn(*const u8, *const u8) -> i32) {
    if n < 2 || sz == 0 {
        return;
    }
    let copy: Vec<u8> = std::slice::from_raw_parts(base, n * sz).to_vec();
    let mut idx: Vec<usize> = (0..n).collect();
    idx.sort_by(|&a, &b| {
        let r = cmp(copy.as_ptr().add(a * sz), copy.as_ptr().add(b * sz));
        r.cmp(&0)
    });
    for (k, &i) in idx.iter().enumerate() {
        std::ptr::copy_nonoverlapping(copy.as_ptr().add(i * sz), base.add(k * sz), sz);
    }
}

unsafe extern "sysv64" fn bsearch(key: *const u8, base: *const u8, n: usize, sz: usize, cmp: unsafe extern "sysv64" fn(*const u8, *const u8) -> i32) -> *mut u8 {
    let (mut lo, mut hi) = (0usize, n);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let p = base.add(mid * sz);
        let r = cmp(key, p);
        if r == 0 {
            return p as *mut u8;
        } else if r < 0 {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    std::ptr::null_mut()
}

static RAND_STATE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

unsafe extern "sysv64" fn srand(seed: u32) {
    RAND_STATE.store(seed as u64, Ordering::Relaxed);
}

unsafe extern "sysv64" fn rand() -> i32 {
    let mut s = RAND_STATE.load(Ordering::Relaxed);
    s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    RAND_STATE.store(s, Ordering::Relaxed);
    ((s >> 33) & 0x7fff_ffff) as i32
}

unsafe extern "sysv64" fn exit(code: i32) -> ! {
    std::process::exit(code);
}

unsafe extern "sysv64" fn abort() -> ! {
    eprintln!("winart: native code called abort()");
    std::process::exit(134);
}

unsafe extern "sysv64" fn stack_chk_fail() -> ! {
    eprintln!("winart: native stack smashing detected");
    std::process::exit(134);
}

unsafe extern "sysv64" fn assert2(file: *const u8, line: i32, func: *const u8, expr: *const u8) -> ! {
    eprintln!("winart: native assertion failed: {} ({}:{} {})", cstring(expr), cstring(file), line, cstring(func));
    std::process::exit(134);
}

unsafe extern "sysv64" fn ret0() -> i32 {
    0
}

unsafe extern "sysv64" fn ret_null() -> *mut u8 {
    std::ptr::null_mut()
}

thread_local! {
    static ERRNO: Cell<i32> = const { Cell::new(0) };
    static PKEYS: RefCell<HashMap<u32, usize>> = RefCell::new(HashMap::new());
}

static NEXT_KEY: AtomicU32 = AtomicU32::new(1);

unsafe extern "sysv64" fn errno_loc() -> *mut i32 {
    ERRNO.with(|c| c.as_ptr())
}

unsafe extern "sysv64" fn strerror(e: i32) -> *const u8 {
    match e {
        0 => b"Success\0".as_ptr(),
        2 => b"No such file or directory\0".as_ptr(),
        12 => b"Out of memory\0".as_ptr(),
        22 => b"Invalid argument\0".as_ptr(),
        _ => b"Unknown error\0".as_ptr(),
    }
}

unsafe extern "sysv64" fn getpid() -> i32 {
    GetCurrentProcessId() as i32
}

unsafe extern "sysv64" fn gettid() -> i32 {
    GetCurrentThreadId() as i32
}

unsafe extern "sysv64" fn getuid() -> u32 {
    10000
}

unsafe extern "sysv64" fn sysconf(name: i32) -> i64 {
    match name {
        39 | 40 => 4096,
        96 | 97 | 95 => std::thread::available_parallelism().map(|n| n.get() as i64).unwrap_or(1),
        _ => -1,
    }
}

unsafe extern "sysv64" fn pthread_self() -> usize {
    GetCurrentThreadId() as usize
}

unsafe extern "sysv64" fn pthread_equal(a: usize, b: usize) -> i32 {
    (a == b) as i32
}

unsafe extern "sysv64" fn pthread_once(once: *mut i32, f: unsafe extern "sysv64" fn()) -> i32 {
    if *once == 0 {
        *once = 1;
        f();
    }
    0
}

unsafe extern "sysv64" fn pthread_key_create(key: *mut u32, _dtor: usize) -> i32 {
    *key = NEXT_KEY.fetch_add(1, Ordering::SeqCst);
    0
}

unsafe extern "sysv64" fn pthread_getspecific(key: u32) -> usize {
    PKEYS.with(|m| m.borrow().get(&key).copied().unwrap_or(0))
}

unsafe extern "sysv64" fn pthread_setspecific(key: u32, v: usize) -> i32 {
    PKEYS.with(|m| m.borrow_mut().insert(key, v));
    0
}

unsafe extern "sysv64" fn pthread_key_delete(key: u32) -> i32 {
    PKEYS.with(|m| m.borrow_mut().remove(&key));
    0
}

unsafe extern "sysv64" fn pthread_create(_t: *mut usize, _a: usize, _f: usize, _arg: usize) -> i32 {
    eprintln!("winart: native pthread_create is not supported");
    11
}

unsafe extern "sysv64" fn pthread_join(_t: usize, _r: usize) -> i32 {
    3
}

fn nap(ns: u64) {
    crate::threads::gil_release();
    std::thread::sleep(std::time::Duration::from_nanos(ns));
    crate::threads::gil_acquire();
}

unsafe extern "sysv64" fn usleep(us: u32) -> i32 {
    nap(us as u64 * 1000);
    0
}

unsafe extern "sysv64" fn sleep(s: u32) -> u32 {
    nap(s as u64 * 1_000_000_000);
    0
}

unsafe extern "sysv64" fn nanosleep(req: *const i64, _rem: *mut i64) -> i32 {
    nap((*req).max(0) as u64 * 1_000_000_000 + (*req.add(1)).max(0) as u64);
    0
}

unsafe extern "sysv64" fn sched_yield() -> i32 {
    std::thread::yield_now();
    0
}

fn mono_ns() -> u64 {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_nanos() as u64 + 1_000_000_000
}

unsafe extern "sysv64" fn time(t: *mut i64) -> i64 {
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    if !t.is_null() {
        *t = s;
    }
    s
}

unsafe extern "sysv64" fn gettimeofday(tv: *mut i64, _tz: *mut u8) -> i32 {
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    if !tv.is_null() {
        *tv = d.as_secs() as i64;
        *tv.add(1) = d.subsec_micros() as i64;
    }
    0
}

unsafe extern "sysv64" fn clock_gettime(clk: i32, ts: *mut i64) -> i32 {
    if ts.is_null() {
        return -1;
    }
    if clk == 0 || clk == 5 {
        let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        *ts = d.as_secs() as i64;
        *ts.add(1) = d.subsec_nanos() as i64;
    } else {
        let n = mono_ns();
        *ts = (n / 1_000_000_000) as i64;
        *ts.add(1) = (n % 1_000_000_000) as i64;
    }
    0
}

unsafe extern "sysv64" fn mmap(addr: *mut u8, len: usize, _prot: i32, flags: i32, _fd: i32, _off: i64) -> *mut u8 {
    if flags & 0x20 == 0 {
        return usize::MAX as *mut u8;
    }
    let p = VirtualAlloc(addr, len, 0x3000, 0x40);
    if p.is_null() {
        usize::MAX as *mut u8
    } else {
        p
    }
}

unsafe extern "sysv64" fn munmap(addr: *mut u8, _len: usize) -> i32 {
    VirtualFree(addr, 0, 0x8000);
    0
}

thread_local! {
    static DLERR: RefCell<Option<std::ffi::CString>> = const { RefCell::new(None) };
}

fn set_dlerr(s: &str) {
    DLERR.with(|e| *e.borrow_mut() = std::ffi::CString::new(s).ok());
}

unsafe extern "sysv64" fn dlopen(name: *const u8, _flags: i32) -> usize {
    if name.is_null() {
        return 1;
    }
    let n = cstring(name);
    let file = n.rsplit(['/', '\\']).next().unwrap_or(&n).to_string();
    if is_system_lib(&file) {
        return 2;
    }
    let g = crate::elf::LIBS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(l) = g.iter().find(|l| l.name == file) {
        return l.id + 0x1000;
    }
    drop(g);
    set_dlerr(&format!("dlopen failed: library \"{}\" not found", n));
    0
}

unsafe extern "sysv64" fn dlsym(handle: usize, name: *const u8) -> usize {
    let n = cstring(name);
    if handle >= 0x1000 && handle != usize::MAX {
        let g = crate::elf::LIBS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(l) = g.iter().find(|l| l.id + 0x1000 == handle) {
            if let Some(&a) = l.syms.get(&n) {
                return a;
            }
        }
    } else if let Some(a) = lookup(&n).or_else(|| crate::elf::find_symbol(&n)) {
        return a;
    }
    set_dlerr(&format!("undefined symbol: {}", n));
    0
}

unsafe extern "sysv64" fn dlclose(_h: usize) -> i32 {
    0
}

unsafe extern "sysv64" fn dlerror() -> *const u8 {
    DLERR.with(|e| match e.borrow_mut().take() {
        Some(c) => c.into_raw() as *const u8,
        None => std::ptr::null(),
    })
}

unsafe extern "sysv64" fn dl_iterate_phdr(cb: unsafe extern "sysv64" fn(*const u8, usize, usize) -> i32, data: usize) -> i32 {
    let libs: Vec<(usize, usize, usize, String)> = {
        let g = crate::elf::LIBS.lock().unwrap_or_else(|e| e.into_inner());
        g.iter().filter(|l| l.phdr != 0).map(|l| (l.base, l.phdr, l.phnum, l.name.clone())).collect()
    };
    for (base, phdr, phnum, name) in libs {
        let mut info = [0u64; 8];
        let mut nm = name.into_bytes();
        nm.push(0);
        let leaked: &'static [u8] = Box::leak(nm.into_boxed_slice());
        info[0] = base as u64;
        info[1] = leaked.as_ptr() as u64;
        info[2] = phdr as u64;
        info[3] = phnum as u64;
        let r = cb(info.as_ptr() as *const u8, 64, data);
        if r != 0 {
            return r;
        }
    }
    0
}

unsafe extern "sysv64" fn tls_get_addr(ti: *const usize) -> usize {
    crate::elf::tls_addr(*ti, *ti.add(1))
}

pub fn is_system_lib(file: &str) -> bool {
    matches!(
        file,
        "libc.so"
            | "libm.so"
            | "libdl.so"
            | "liblog.so"
            | "libstdc++.so"
            | "libandroid.so"
            | "libz.so"
            | "libjnigraphics.so"
            | "libOpenSLES.so"
            | "libEGL.so"
            | "libGLESv1_CM.so"
            | "libGLESv2.so"
            | "libGLESv3.so"
            | "libvulkan.so"
            | "libmediandk.so"
            | "libnativewindow.so"
            | "libcamera2ndk.so"
            | "libaaudio.so"
            | "libbinder_ndk.so"
            | "libsync.so"
            | "libamidi.so"
    )
}

unsafe extern "sysv64" fn memmove(d: *mut u8, s: *const u8, n: usize) -> *mut u8 {
    std::ptr::copy(s, d, n);
    d
}

macro_rules! m1 {
    ($($n:ident => $m:ident),* $(,)?) => { $( unsafe extern "sysv64" fn $n(x: f64) -> f64 { x.$m() } )* };
}

macro_rules! m1f {
    ($($n:ident => $m:ident),* $(,)?) => { $( unsafe extern "sysv64" fn $n(x: f32) -> f32 { x.$m() } )* };
}

macro_rules! m2 {
    ($($n:ident => $m:ident),* $(,)?) => { $( unsafe extern "sysv64" fn $n(x: f64, y: f64) -> f64 { x.$m(y) } )* };
}

macro_rules! m2f {
    ($($n:ident => $m:ident),* $(,)?) => { $( unsafe extern "sysv64" fn $n(x: f32, y: f32) -> f32 { x.$m(y) } )* };
}

m1!(sin => sin, cos => cos, tan => tan, asin => asin, acos => acos, atan => atan, sinh => sinh, cosh => cosh, tanh => tanh, asinh => asinh, acosh => acosh, atanh => atanh, exp => exp, exp2 => exp2, expm1 => exp_m1, log => ln, log2 => log2, log10 => log10, log1p => ln_1p, sqrt => sqrt, cbrt => cbrt, floor => floor, ceil => ceil, round => round, trunc => trunc, fabs => abs, rint => round_ties_even, nearbyint => round_ties_even);
m1f!(sinf => sin, cosf => cos, tanf => tan, asinf => asin, acosf => acos, atanf => atan, sinhf => sinh, coshf => cosh, tanhf => tanh, expf => exp, exp2f => exp2, logf => ln, log2f => log2, log10f => log10, sqrtf => sqrt, cbrtf => cbrt, floorf => floor, ceilf => ceil, roundf => round, truncf => trunc, fabsf => abs, rintf => round_ties_even);
m2!(pow => powf, atan2 => atan2, hypot => hypot, fmin => min, fmax => max, copysign => copysign);
m2f!(powf => powf, atan2f => atan2, hypotf => hypot, fminf => min, fmaxf => max, copysignf => copysign);

unsafe extern "sysv64" fn fmod(x: f64, y: f64) -> f64 {
    x % y
}

unsafe extern "sysv64" fn fmodf(x: f32, y: f32) -> f32 {
    x % y
}

unsafe extern "sysv64" fn ldexp(x: f64, e: i32) -> f64 {
    x * 2f64.powi(e)
}

unsafe extern "sysv64" fn modf(x: f64, ip: *mut f64) -> f64 {
    let t = x.trunc();
    *ip = t;
    x - t
}

unsafe extern "sysv64" fn lround(x: f64) -> i64 {
    x.round() as i64
}

unsafe extern "sysv64" fn lrint(x: f64) -> i64 {
    x.round_ties_even() as i64
}

unsafe extern "sysv64" fn isnan(x: f64) -> i32 {
    x.is_nan() as i32
}

unsafe extern "sysv64" fn isinf(x: f64) -> i32 {
    x.is_infinite() as i32
}

unsafe extern "sysv64" fn isnanf(x: f32) -> i32 {
    x.is_nan() as i32
}

unsafe extern "sysv64" fn isinff(x: f32) -> i32 {
    x.is_infinite() as i32
}

macro_rules! vstub {
    ($name:ident, $gpoff:literal, $reg:literal, $target:path) => {
        std::arch::global_asm!(
            ".text",
            ".p2align 4",
            concat!(".globl ", stringify!($name)),
            concat!(stringify!($name), ":"),
            "push rbp",
            "mov rbp, rsp",
            "sub rsp, 208",
            "mov qword ptr [rsp], rdi",
            "mov qword ptr [rsp + 8], rsi",
            "mov qword ptr [rsp + 16], rdx",
            "mov qword ptr [rsp + 24], rcx",
            "mov qword ptr [rsp + 32], r8",
            "mov qword ptr [rsp + 40], r9",
            "test al, al",
            "je 2f",
            "movaps xmmword ptr [rsp + 48], xmm0",
            "movaps xmmword ptr [rsp + 64], xmm1",
            "movaps xmmword ptr [rsp + 80], xmm2",
            "movaps xmmword ptr [rsp + 96], xmm3",
            "movaps xmmword ptr [rsp + 112], xmm4",
            "movaps xmmword ptr [rsp + 128], xmm5",
            "movaps xmmword ptr [rsp + 144], xmm6",
            "movaps xmmword ptr [rsp + 160], xmm7",
            "2:",
            concat!("mov dword ptr [rsp + 176], ", $gpoff),
            "mov dword ptr [rsp + 180], 48",
            "lea rax, [rbp + 16]",
            "mov qword ptr [rsp + 184], rax",
            "mov qword ptr [rsp + 192], rsp",
            concat!("lea ", $reg, ", [rsp + 176]"),
            "call {t}",
            "movq xmm0, rax",
            "leave",
            "ret",
            t = sym $target,
        );
        extern "sysv64" {
            pub fn $name();
        }
    };
}

pub(crate) use vstub;

vstub!(winart_va_snprintf, 24, "rcx", vsnprintf);
vstub!(winart_va_sprintf, 16, "rdx", vsprintf);
vstub!(winart_va_printf, 8, "rsi", vprintf);
vstub!(winart_va_log_print, 24, "rcx", android_log_vprint);
vstub!(winart_va_snprintf_chk, 40, "r9", vsnprintf_chk);
vstub!(winart_va_sprintf_chk, 32, "r8", vsprintf_chk);

pub fn lookup(name: &str) -> Option<usize> {
    macro_rules! t {
        ($($n:ident),* $(,)?) => { match name { $( stringify!($n) => return Some($n as usize), )* _ => {} } };
    }
    macro_rules! p {
        ($($s:literal => $f:expr),* $(,)?) => { match name { $( $s => return Some($f as usize), )* _ => {} } };
    }
    t!(
        malloc, calloc, realloc, free, memalign, posix_memalign, malloc_usable_size, memcpy, memmove, memset, memcmp, memchr, memrchr, strlen, strnlen, strcmp, strncmp, strcasecmp, strncasecmp, strcpy, strncpy, strcat, strncat, strchr, strrchr, strstr, strspn, strcspn,
        strpbrk, strdup, strndup, strtok, toupper, tolower, isalpha, isdigit, isalnum, isspace, isupper, islower, isxdigit, ispunct, isprint, vsnprintf, vsprintf, vprintf, puts, putchar, fflush, strtoll, strtoull, atoi, atol, strtod, strtof, atof, labs, qsort, bsearch,
        srand, rand, exit, abort, strerror, getpid, gettid, getuid, sysconf, pthread_self, pthread_equal, pthread_once, pthread_key_create, pthread_key_delete, pthread_getspecific, pthread_setspecific, pthread_create, pthread_join, usleep, sleep, nanosleep, sched_yield, time, gettimeofday,
        clock_gettime, mmap, munmap, dlopen, dlsym, dlclose, dlerror, dl_iterate_phdr, sin, cos, tan, asin, acos, atan, sinh, cosh, tanh, asinh, acosh, atanh, exp, exp2, expm1, log, log2, log10, log1p, sqrt, cbrt, floor, ceil, round, trunc, fabs, rint, nearbyint, sinf,
        cosf, tanf, asinf, acosf, atanf, sinhf, coshf, tanhf, expf, exp2f, logf, log2f, log10f, sqrtf, cbrtf, floorf, ceilf, roundf, truncf, fabsf, rintf, pow, atan2, hypot, fmin, fmax, copysign, powf, atan2f, hypotf, fminf, fmaxf, copysignf, fmod, fmodf, ldexp, modf,
        lround, lrint, isnan, isinf, isnanf, isinff
    );
    p!(
        "strtol" => strtoll,
        "strtoul" => strtoull,
        "atoll" => atol,
        "llabs" => labs,
        "abs" => abs_i,
        "random" => rand,
        "srandom" => srand,
        "_exit" => exit,
        "llround" => lround,
        "llrint" => lrint,
        "scalbn" => ldexp,
        "__isnan" => isnan,
        "__isinf" => isinf,
        "__isnanf" => isnanf,
        "__isinff" => isinff,
        "__errno" => errno_loc,
        "__stack_chk_fail" => stack_chk_fail,
        "__assert2" => assert2,
        "__strlen_chk" => strlen_chk,
        "__memcpy_chk" => memcpy_chk,
        "__memmove_chk" => memcpy_chk,
        "__memset_chk" => memset_chk,
        "__strcpy_chk" => strcpy_chk,
        "__strncpy_chk" => strncpy_chk,
        "__strcat_chk" => strcat_chk,
        "__strncat_chk" => strncat_chk,
        "__vsnprintf_chk" => vsnprintf_chk,
        "__vsprintf_chk" => vsprintf_chk,
        "__android_log_vprint" => android_log_vprint,
        "__android_log_write" => android_log_write,
        "__android_log_buf_write" => android_log_buf_write,
        "__android_log_assert" => android_log_assert,
        "__android_log_is_loggable" => android_log_is_loggable,
        "__tls_get_addr" => tls_get_addr,
        "snprintf" => winart_va_snprintf,
        "sprintf" => winart_va_sprintf,
        "printf" => winart_va_printf,
        "__android_log_print" => winart_va_log_print,
        "__snprintf_chk" => winart_va_snprintf_chk,
        "__sprintf_chk" => winart_va_sprintf_chk,
        "getenv" => ret_null,
        "pthread_mutex_init" => ret0,
        "pthread_mutex_destroy" => ret0,
        "pthread_mutex_lock" => ret0,
        "pthread_mutex_unlock" => ret0,
        "pthread_mutex_trylock" => ret0,
        "pthread_mutexattr_init" => ret0,
        "pthread_mutexattr_destroy" => ret0,
        "pthread_mutexattr_settype" => ret0,
        "pthread_cond_init" => ret0,
        "pthread_cond_destroy" => ret0,
        "pthread_cond_signal" => ret0,
        "pthread_cond_broadcast" => ret0,
        "pthread_cond_wait" => ret0,
        "pthread_cond_timedwait" => ret0,
        "pthread_attr_init" => ret0,
        "pthread_attr_destroy" => ret0,
        "pthread_attr_setdetachstate" => ret0,
        "pthread_attr_setstacksize" => ret0,
        "pthread_detach" => ret0,
        "pthread_setname_np" => ret0,
        "__cxa_atexit" => ret0,
        "__cxa_finalize" => ret0,
        "__cxa_thread_atexit_impl" => ret0,
        "__register_atfork" => ret0,
        "atexit" => ret0,
        "mprotect" => ret0,
        "madvise" => ret0,
        "prctl" => ret0,
        "setlocale" => ret_null
    );
    None
}
