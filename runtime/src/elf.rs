use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[link(name = "kernel32")]
extern "system" {
    fn VirtualAlloc(addr: *mut u8, size: usize, ty: u32, prot: u32) -> *mut u8;
}

pub struct Lib {
    pub name: String,
    pub base: usize,
    pub size: usize,
    pub syms: HashMap<String, usize>,
    pub patched: usize,
    pub unresolved: Vec<String>,
    pub phdr: usize,
    pub phnum: usize,
    pub id: usize,
    pub init: (u64, (u64, u64)),
}

struct Seg {
    vaddr: u64,
    off: u64,
    filesz: u64,
    memsz: u64,
    flags: u32,
}

struct Parsed {
    segs: Vec<Seg>,
    dynamic: Option<u64>,
    phoff: u64,
    phnum: usize,
    phentsize: usize,
    tls: Option<(u64, u64, u64)>,
}

#[derive(Default)]
struct DynInfo {
    needed: Vec<u64>,
    strtab: u64,
    symtab: u64,
    hash: u64,
    gnu_hash: u64,
    rela: (u64, u64),
    jmprel: (u64, u64),
    init: u64,
    init_array: (u64, u64),
    relr: (u64, u64),
    arela: (u64, u64),
    rel: bool,
}

struct Sym {
    name: String,
    info: u8,
    shndx: u16,
    value: u64,
}

struct View<'a> {
    buf: &'a [u8],
    segs: Option<&'a [Seg]>,
}

fn u16le(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}

fn u32le(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

fn u64le(d: &[u8], o: usize) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&d[o..o + 8]);
    u64::from_le_bytes(b)
}

impl<'a> View<'a> {
    fn off(&self, vaddr: u64) -> Result<usize, String> {
        match self.segs {
            None => {
                if (vaddr as usize) < self.buf.len() {
                    Ok(vaddr as usize)
                } else {
                    Err(format!("address {:#x} outside image", vaddr))
                }
            }
            Some(segs) => {
                for s in segs {
                    if vaddr >= s.vaddr && vaddr < s.vaddr + s.filesz {
                        let o = (s.off + (vaddr - s.vaddr)) as usize;
                        if o < self.buf.len() {
                            return Ok(o);
                        }
                    }
                }
                Err(format!("address {:#x} not in a loadable segment", vaddr))
            }
        }
    }

    fn cstr(&self, vaddr: u64) -> Result<String, String> {
        let o = self.off(vaddr)?;
        let mut e = o;
        while e < self.buf.len() && self.buf[e] != 0 {
            e += 1;
        }
        Ok(String::from_utf8_lossy(&self.buf[o..e]).into_owned())
    }
}

fn parse_header(d: &[u8]) -> Result<Parsed, String> {
    if d.len() < 64 || &d[0..4] != b"\x7fELF" {
        return Err("not an ELF file".into());
    }
    if d[4] != 2 {
        return Err("32-bit ELF library (armeabi-v7a or x86) is not supported".into());
    }
    if d[5] != 1 {
        return Err("big-endian ELF is not supported".into());
    }
    let machine = u16le(d, 18);
    match machine {
        62 => {}
        183 => return Err("AArch64 library: running it needs ARM64 CPU emulation (not implemented)".into()),
        m => return Err(format!("unsupported ELF machine {}", m)),
    }
    if u16le(d, 16) != 3 {
        return Err("not a shared object (ET_DYN)".into());
    }
    let phoff = u64le(d, 32) as usize;
    let phentsize = u16le(d, 54) as usize;
    let phnum = u16le(d, 56) as usize;
    let mut segs = Vec::new();
    let mut dynamic = None;
    let mut tls = None;
    for i in 0..phnum {
        let p = phoff + i * phentsize;
        if p + 56 > d.len() {
            return Err("truncated program headers".into());
        }
        let ty = u32le(d, p);
        let flags = u32le(d, p + 4);
        let off = u64le(d, p + 8);
        let vaddr = u64le(d, p + 16);
        let filesz = u64le(d, p + 32);
        let memsz = u64le(d, p + 40);
        if ty == 1 {
            if off + filesz > d.len() as u64 {
                return Err("segment beyond end of file".into());
            }
            segs.push(Seg { vaddr, off, filesz, memsz, flags });
        } else if ty == 2 {
            dynamic = Some(vaddr);
        } else if ty == 7 {
            tls = Some((vaddr, filesz, memsz));
        }
    }
    if segs.is_empty() {
        return Err("no loadable segments".into());
    }
    Ok(Parsed { segs, dynamic, phoff: phoff as u64, phnum, phentsize, tls })
}

fn parse_dyn(v: &View, dyn_vaddr: u64) -> Result<DynInfo, String> {
    let mut o = v.off(dyn_vaddr)?;
    let mut di = DynInfo::default();
    let mut sz: (u64, u64, u64) = (0, 0, 0);
    loop {
        if o + 16 > v.buf.len() {
            return Err("truncated dynamic section".into());
        }
        let tag = u64le(v.buf, o) as i64;
        let val = u64le(v.buf, o + 8);
        o += 16;
        match tag {
            0 => break,
            1 => di.needed.push(val),
            2 => sz.1 = val,
            4 => di.hash = val,
            5 => di.strtab = val,
            6 => di.symtab = val,
            7 => di.rela.0 = val,
            8 => di.rela.1 = val,
            12 => di.init = val,
            17 | 18 => di.rel = true,
            23 => di.jmprel.0 = val,
            25 => di.init_array.0 = val,
            27 => di.init_array.1 = val,
            35 => di.relr.1 = val,
            36 => di.relr.0 = val,
            0x6ffffef5 => di.gnu_hash = val,
            0x60000011 => di.arela.0 = val,
            0x60000012 => di.arela.1 = val,
            0x6000000f | 0x60000010 => di.rel = true,
            _ => {}
        }
    }
    di.jmprel.1 = sz.1;
    let _ = sz.0 + sz.2;
    Ok(di)
}

fn nsyms(v: &View, di: &DynInfo) -> Result<usize, String> {
    if di.hash != 0 {
        let o = v.off(di.hash)?;
        return Ok(u32le(v.buf, o + 4) as usize);
    }
    if di.gnu_hash != 0 {
        let o = v.off(di.gnu_hash)?;
        let nb = u32le(v.buf, o) as usize;
        let symoff = u32le(v.buf, o + 4) as usize;
        let bloom = u32le(v.buf, o + 8) as usize;
        let buckets = o + 16 + bloom * 8;
        let chains = buckets + nb * 4;
        let mut maxs = 0usize;
        for i in 0..nb {
            let b = u32le(v.buf, buckets + i * 4) as usize;
            if b > maxs {
                maxs = b;
            }
        }
        if maxs < symoff {
            return Ok(symoff);
        }
        let mut idx = maxs;
        loop {
            let c = u32le(v.buf, chains + (idx - symoff) * 4);
            if c & 1 == 1 {
                break;
            }
            idx += 1;
        }
        return Ok(idx + 1);
    }
    Err("library has no symbol hash table".into())
}

fn symbols(v: &View, di: &DynInfo) -> Result<Vec<Sym>, String> {
    if di.symtab == 0 || di.strtab == 0 {
        return Ok(Vec::new());
    }
    let n = nsyms(v, di)?;
    let so = v.off(di.symtab)?;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let p = so + i * 24;
        if p + 24 > v.buf.len() {
            break;
        }
        let name_off = u32le(v.buf, p) as u64;
        let name = v.cstr(di.strtab + name_off)?;
        out.push(Sym { name, info: v.buf[p + 4], shndx: u16le(v.buf, p + 6), value: u64le(v.buf, p + 8) });
    }
    Ok(out)
}

pub fn needed(d: &[u8]) -> Result<Vec<String>, String> {
    let p = parse_header(d)?;
    let dv = p.dynamic.ok_or("no dynamic section")?;
    let v = View { buf: d, segs: Some(&p.segs) };
    let di = parse_dyn(&v, dv)?;
    let mut out = Vec::new();
    for &n in &di.needed {
        out.push(v.cstr(di.strtab + n)?);
    }
    Ok(out)
}

pub fn imports(d: &[u8]) -> Result<Vec<(String, bool)>, String> {
    let p = parse_header(d)?;
    let dv = p.dynamic.ok_or("no dynamic section")?;
    let v = View { buf: d, segs: Some(&p.segs) };
    let di = parse_dyn(&v, dv)?;
    let mut out = Vec::new();
    for s in symbols(&v, &di)? {
        if s.shndx == 0 && !s.name.is_empty() {
            out.push((s.name, s.info >> 4 == 2));
        }
    }
    Ok(out)
}

pub fn arch_of(d: &[u8]) -> String {
    if d.len() < 20 || &d[0..4] != b"\x7fELF" {
        return "not-elf".into();
    }
    match u16le(d, 18) {
        62 => "x86_64".into(),
        183 => "aarch64".into(),
        40 => "arm".into(),
        3 => "x86".into(),
        m => format!("machine-{}", m),
    }
}

struct Stubs {
    page: usize,
    used: usize,
    cap: usize,
    names: Vec<String>,
}

static STUBS: Mutex<Stubs> = Mutex::new(Stubs { page: 0, used: 0, cap: 0, names: Vec::new() });

extern "sysv64" fn unresolved_trap(id: usize) -> ! {
    let name = STUBS.lock().ok().and_then(|s| s.names.get(id).cloned()).unwrap_or_default();
    eprintln!("winart: native code called unresolved symbol '{}'", name);
    std::process::exit(120);
}

pub fn stub_for(name: &str) -> usize {
    let mut s = STUBS.lock().unwrap_or_else(|e| e.into_inner());
    if s.page == 0 || s.used + 32 > s.cap {
        let cap = 64 * 1024;
        let p = unsafe { VirtualAlloc(std::ptr::null_mut(), cap, 0x3000, 0x40) };
        if p.is_null() {
            return 0;
        }
        s.page = p as usize;
        s.used = 0;
        s.cap = cap;
    }
    let id = s.names.len();
    s.names.push(name.to_string());
    let at = s.page + s.used;
    s.used += 32;
    let mut code = [0x90u8; 32];
    code[0] = 0x48;
    code[1] = 0xBF;
    code[2..10].copy_from_slice(&(id as u64).to_le_bytes());
    code[10] = 0x48;
    code[11] = 0xB8;
    code[12..20].copy_from_slice(&(unresolved_trap as usize as u64).to_le_bytes());
    code[20] = 0xFF;
    code[21] = 0xE0;
    unsafe {
        std::ptr::copy_nonoverlapping(code.as_ptr(), at as *mut u8, 32);
    }
    at
}

fn patch_fs(buf: &mut [u8]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i + 9 <= buf.len() {
        if buf[i] == 0x64
            && (buf[i + 1] == 0x48 || buf[i + 1] == 0x4c)
            && buf[i + 3] & 0xc7 == 0x04
            && buf[i + 4] == 0x25
            && buf[i + 5] == 0x28
            && buf[i + 6] == 0
            && buf[i + 7] == 0
            && buf[i + 8] == 0
        {
            let rexr = buf[i + 1] & 4 != 0;
            let reg = (buf[i + 3] >> 3) & 7;
            let rx = if rexr { 0x4d } else { 0x48 };
            let op = buf[i + 2];
            let new_op = match op {
                0x8b => 0x31u8,
                0x3b | 0x2b | 0x33 | 0x39 | 0x29 | 0x31 => 0x85u8,
                _ => {
                    i += 1;
                    continue;
                }
            };
            buf[i] = rx;
            buf[i + 1] = new_op;
            buf[i + 2] = 0xC0 | (reg << 3) | reg;
            for k in 3..9 {
                buf[i + k] = 0x90;
            }
            n += 1;
            i += 9;
            continue;
        }
        i += 1;
    }
    n
}

fn call0(addr: usize) {
    let f: extern "sysv64" fn(i32, usize, usize) = unsafe { std::mem::transmute(addr) };
    f(0, 0, 0);
}

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);
static TLS_MODS: Mutex<Option<HashMap<usize, (usize, usize, usize)>>> = Mutex::new(None);
pub static LIBS: Mutex<Vec<Lib>> = Mutex::new(Vec::new());

thread_local! {
    static TLS_BLOCKS: RefCell<HashMap<usize, Box<[u8]>>> = RefCell::new(HashMap::new());
}

pub fn tls_addr(id: usize, off: usize) -> usize {
    let info = {
        let g = TLS_MODS.lock().unwrap_or_else(|e| e.into_inner());
        g.as_ref().and_then(|m| m.get(&id).copied())
    };
    let (init, filesz, memsz) = match info {
        Some(x) => x,
        None => return 0,
    };
    TLS_BLOCKS.with(|b| {
        let mut b = b.borrow_mut();
        let blk = b.entry(id).or_insert_with(|| {
            let mut v = vec![0u8; memsz.max(1) + 64].into_boxed_slice();
            unsafe {
                std::ptr::copy_nonoverlapping(init as *const u8, v.as_mut_ptr(), filesz.min(memsz));
            }
            v
        });
        blk.as_ptr() as usize + off
    })
}

pub fn find_symbol(name: &str) -> Option<usize> {
    let g = LIBS.lock().unwrap_or_else(|e| e.into_inner());
    for l in g.iter() {
        if let Some(&a) = l.syms.get(name) {
            return Some(a);
        }
    }
    None
}

pub fn is_loaded(name: &str) -> bool {
    let g = LIBS.lock().unwrap_or_else(|e| e.into_inner());
    g.iter().any(|l| l.name == name)
}

fn sleb(d: &[u8], pos: &mut usize) -> Result<i64, String> {
    let mut result: i64 = 0;
    let mut shift = 0u32;
    loop {
        let b = *d.get(*pos).ok_or("truncated packed relocations")?;
        *pos += 1;
        if shift < 64 {
            result |= ((b & 0x7f) as i64) << shift;
        }
        shift += 7;
        if b & 0x80 == 0 {
            if shift < 64 && b & 0x40 != 0 {
                result |= -1i64 << shift;
            }
            break;
        }
        if shift > 70 {
            return Err("bad packed relocation encoding".into());
        }
    }
    Ok(result)
}

fn decode_aps2(buf: &[u8]) -> Result<Vec<(u64, u64, i64)>, String> {
    if buf.len() < 4 || &buf[0..4] != b"APS2" {
        return Err("unknown packed relocation format".into());
    }
    let mut pos = 4;
    let count = sleb(buf, &mut pos)? as usize;
    let mut off = sleb(buf, &mut pos)? as u64;
    let mut out = Vec::with_capacity(count.min(1 << 24));
    let mut info = 0u64;
    let mut addend = 0i64;
    let mut done = 0usize;
    while done < count {
        let gsize = sleb(buf, &mut pos)? as usize;
        let flags = sleb(buf, &mut pos)? as u64;
        let by_info = flags & 1 != 0;
        let by_delta = flags & 2 != 0;
        let by_addend = flags & 4 != 0;
        let has_addend = flags & 8 != 0;
        let mut gdelta = 0u64;
        if by_delta {
            gdelta = sleb(buf, &mut pos)? as u64;
        }
        if by_info {
            info = sleb(buf, &mut pos)? as u64;
        }
        if has_addend && by_addend {
            addend = addend.wrapping_add(sleb(buf, &mut pos)?);
        }
        if !has_addend {
            addend = 0;
        }
        if gsize == 0 {
            return Err("empty packed relocation group".into());
        }
        for _ in 0..gsize {
            if by_delta {
                off = off.wrapping_add(gdelta);
            } else {
                off = off.wrapping_add(sleb(buf, &mut pos)? as u64);
            }
            if !by_info {
                info = sleb(buf, &mut pos)? as u64;
            }
            if has_addend && !by_addend {
                addend = addend.wrapping_add(sleb(buf, &mut pos)?);
            }
            out.push((off, info, addend));
        }
        done += gsize;
    }
    Ok(out)
}

pub fn load(name: &str, d: &[u8], resolve: &mut dyn FnMut(&str) -> Option<usize>) -> Result<Lib, String> {
    let p = parse_header(d)?;
    let dv = p.dynamic.ok_or("no dynamic section")?;
    let mut max_va = 0u64;
    for s in &p.segs {
        max_va = max_va.max(s.vaddr + s.memsz);
    }
    let size = ((max_va + 0xfff) & !0xfff) as usize;
    let mem = unsafe { VirtualAlloc(std::ptr::null_mut(), size, 0x3000, 0x40) };
    if mem.is_null() {
        return Err("VirtualAlloc failed".into());
    }
    let base = mem as usize;
    for s in &p.segs {
        unsafe {
            std::ptr::copy_nonoverlapping(d.as_ptr().add(s.off as usize), mem.add(s.vaddr as usize), s.filesz as usize);
        }
    }
    let img: &mut [u8] = unsafe { std::slice::from_raw_parts_mut(mem, size) };
    let (di, syms) = {
        let v = View { buf: img, segs: None };
        let di = parse_dyn(&v, dv)?;
        let syms = symbols(&v, &di)?;
        (di, syms)
    };
    if di.rel {
        return Err("REL relocations are not supported (x86_64 uses RELA)".into());
    }
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    if let Some((va, fsz, msz)) = p.tls {
        let mut g = TLS_MODS.lock().unwrap_or_else(|e| e.into_inner());
        g.get_or_insert_with(HashMap::new).insert(id, (base + va as usize, fsz as usize, msz as usize));
    }
    let mut own: HashMap<String, usize> = HashMap::new();
    for s in &syms {
        if s.shndx != 0 && s.info & 0xf != 6 && s.info >> 4 != 0 && !s.name.is_empty() {
            own.insert(s.name.clone(), base + s.value as usize);
        }
    }
    let mut unresolved: Vec<String> = Vec::new();
    let mut cache: HashMap<usize, usize> = HashMap::new();
    let mut sym_addr = |idx: usize, unresolved: &mut Vec<String>| -> usize {
        if idx == 0 || idx >= syms.len() {
            return 0;
        }
        if let Some(&a) = cache.get(&idx) {
            return a;
        }
        let s = &syms[idx];
        let a = if s.shndx != 0 && s.info & 0xf != 6 {
            base + s.value as usize
        } else if let Some(a) = resolve(&s.name) {
            a
        } else if s.info >> 4 == 2 {
            0
        } else {
            unresolved.push(s.name.clone());
            stub_for(&s.name)
        };
        cache.insert(idx, a);
        a
    };
    let mut relocs: Vec<(usize, u64, i64)> = Vec::new();
    let mut tables: Vec<(u64, u64)> = Vec::new();
    if di.rela.0 != 0 {
        tables.push(di.rela);
    }
    if di.jmprel.0 != 0 {
        tables.push(di.jmprel);
    }
    for (va, sz) in tables {
        let n = (sz / 24) as usize;
        for i in 0..n {
            let o = va as usize + i * 24;
            if o + 24 > size {
                return Err("relocation table outside image".into());
            }
            relocs.push((u64le(img, o) as usize, u64le(img, o + 8), u64le(img, o + 16) as i64));
        }
    }
    if di.arela.0 != 0 && di.arela.1 != 0 {
        let o = di.arela.0 as usize;
        let e = o + di.arela.1 as usize;
        if e > size {
            return Err("packed relocation table outside image".into());
        }
        for (off, info, add) in decode_aps2(&img[o..e])? {
            relocs.push((off as usize, info, add));
        }
    }
    let mut irel: Vec<(usize, usize)> = Vec::new();
    for (off, info, addend) in relocs {
        let ty = (info & 0xffff_ffff) as u32;
        let si = (info >> 32) as usize;
        if off + 8 > size {
            return Err(format!("relocation target {:#x} outside image", off));
        }
        let val: u64 = match ty {
            0 => continue,
            1 => (sym_addr(si, &mut unresolved) as i64).wrapping_add(addend) as u64,
            6 | 7 => sym_addr(si, &mut unresolved) as u64,
            8 => (base as i64).wrapping_add(addend) as u64,
            37 => {
                irel.push((off, (base as i64 + addend) as usize));
                continue;
            }
            16 => {
                if si == 0 || syms.get(si).map(|s| s.shndx != 0).unwrap_or(false) {
                    id as u64
                } else {
                    unresolved.push(format!("tls:{}", syms.get(si).map(|s| s.name.clone()).unwrap_or_default()));
                    0
                }
            }
            17 => match syms.get(si) {
                Some(s) if si != 0 && s.shndx != 0 => (s.value as i64).wrapping_add(addend) as u64,
                _ if si == 0 => addend as u64,
                _ => {
                    unresolved.push(format!("tls:{}", syms.get(si).map(|s| s.name.clone()).unwrap_or_default()));
                    0
                }
            },
            18 | 36 => {
                let nm = syms.get(si).map(|s| s.name.clone()).unwrap_or_default();
                unresolved.push(format!("tls:{}", nm));
                0
            }
            t => return Err(format!("unsupported relocation type {}", t)),
        };
        img[off..off + 8].copy_from_slice(&val.to_le_bytes());
    }
    if di.relr.0 != 0 {
        let n = (di.relr.1 / 8) as usize;
        let mut at = 0usize;
        for i in 0..n {
            let e = u64le(img, di.relr.0 as usize + i * 8);
            if e & 1 == 0 {
                at = e as usize;
                let cur = u64le(img, at);
                img[at..at + 8].copy_from_slice(&cur.wrapping_add(base as u64).to_le_bytes());
                at += 8;
            } else {
                let mut bits = e >> 1;
                let mut a = at;
                while bits != 0 {
                    if bits & 1 == 1 {
                        let cur = u64le(img, a);
                        img[a..a + 8].copy_from_slice(&cur.wrapping_add(base as u64).to_le_bytes());
                    }
                    bits >>= 1;
                    a += 8;
                }
                at += 63 * 8;
            }
        }
    }
    for (off, target) in irel {
        let f: extern "sysv64" fn() -> usize = unsafe { std::mem::transmute(target) };
        let r = f();
        img[off..off + 8].copy_from_slice(&(r as u64).to_le_bytes());
    }
    let mut patched = 0;
    for s in &p.segs {
        if s.flags & 1 != 0 {
            let a = s.vaddr as usize;
            patched += patch_fs(&mut img[a..a + s.filesz as usize]);
        }
    }
    let mut phdr = 0usize;
    for s in &p.segs {
        let end = p.phoff + (p.phnum as u64) * (p.phentsize as u64);
        if p.phoff >= s.off && end <= s.off + s.filesz {
            phdr = base + (s.vaddr + (p.phoff - s.off)) as usize;
            break;
        }
    }
    let lib = Lib {
        name: name.to_string(),
        base,
        size,
        syms: own,
        patched,
        unresolved,
        phdr,
        phnum: p.phnum,
        id,
        init: (di.init, di.init_array),
    };
    Ok(lib)
}

pub fn run_init(base: usize, size: usize, init: (u64, (u64, u64))) {
    let img: &[u8] = unsafe { std::slice::from_raw_parts(base as *const u8, size) };
    if init.0 != 0 {
        call0(base + init.0 as usize);
    }
    if (init.1).0 != 0 {
        let n = ((init.1).1 / 8) as usize;
        for i in 0..n {
            let f = u64le(img, (init.1).0 as usize + i * 8) as usize;
            if f != 0 && f != usize::MAX {
                call0(f);
            }
        }
    }
}

pub fn register(mut lib: Lib) {
    let (base, size, init) = (lib.base, lib.size, lib.init);
    LIBS.lock().unwrap_or_else(|e| e.into_inner()).push(lib);
    run_init(base, size, init);
}
