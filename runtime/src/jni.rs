#![allow(dead_code, function_casts_as_integer)]
use crate::bionic::{cstr, cstring, free, malloc, vstub, VaList};
use crate::elf;
use crate::runtime::{Kind, Method, Obj, Ref, Runtime, R};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::sync::OnceLock;

pub struct JMethod {
    pub class: usize,
    pub key: String,
    pub params: Vec<u8>,
    pub ret: u8,
    pub is_static: bool,
}

pub struct JField {
    pub class: usize,
    pub slot: usize,
    pub kind: u8,
    pub is_static: bool,
}

#[derive(Default)]
pub struct Jni {
    pub app_dex: Option<usize>,
    pub registered: HashMap<(usize, String), usize>,
    pub bound: HashMap<(usize, String), usize>,
    pub methods: Vec<JMethod>,
    pub mcache: HashMap<(usize, String, bool), usize>,
    pub fields: Vec<JField>,
    pub fcache: HashMap<(usize, String, bool), usize>,
    pub locals: Vec<(u32, Ref)>,
    pub globals: Vec<Ref>,
    pub libs: Vec<String>,
}

pub enum LoadErr {
    Msg(String),
    Exc(Ref),
}

static RT: AtomicPtr<Runtime> = AtomicPtr::new(std::ptr::null_mut());
static ENV: OnceLock<usize> = OnceLock::new();
static VM: OnceLock<usize> = OnceLock::new();

thread_local! {
    static PENDING: Cell<Ref> = const { Cell::new(0) };
    static NDEPTH: Cell<u32> = const { Cell::new(0) };
}

fn rt() -> &'static mut Runtime {
    unsafe { &mut *RT.load(Ordering::Relaxed) }
}

fn tid() -> u32 {
    crate::threads::cur_tid()
}

fn loc(r: Ref) -> usize {
    if r != 0 {
        rt().jni.locals.push((tid(), r));
    }
    r as usize
}

fn set_pending(r: Ref) {
    loc(r);
    PENDING.with(|p| p.set(r));
}

fn take_pending() -> Ref {
    PENDING.with(|p| p.replace(0))
}

fn throw(name: &str, msg: &str) {
    let e = rt().throw_msg(name, msg);
    set_pending(e);
}

fn class_idx(h: usize) -> usize {
    match rt().heap.get(h).map(|o| &o.kind) {
        Some(Kind::Class(c)) => *c,
        _ => 0,
    }
}

pub fn parse_sig(key: &str) -> (Vec<u8>, u8) {
    let b = key.as_bytes();
    let mut i = key.find('(').map(|x| x + 1).unwrap_or(0);
    let mut params = Vec::new();
    while i < b.len() && b[i] != b')' {
        match b[i] {
            b'L' => {
                while i < b.len() && b[i] != b';' {
                    i += 1;
                }
                params.push(b'L');
                i += 1;
            }
            b'[' => {
                while i < b.len() && b[i] == b'[' {
                    i += 1;
                }
                if i < b.len() && b[i] == b'L' {
                    while i < b.len() && b[i] != b';' {
                        i += 1;
                    }
                }
                params.push(b'[');
                i += 1;
            }
            c => {
                params.push(c);
                i += 1;
            }
        }
    }
    let ret = b.get(i + 1).copied().unwrap_or(b'V');
    (params, ret)
}

fn mutf8_decode(b: &[u8]) -> Vec<u16> {
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c < 0x80 {
            out.push(c as u16);
            i += 1;
        } else if c & 0xe0 == 0xc0 && i + 1 < b.len() {
            out.push((((c & 0x1f) as u16) << 6) | (b[i + 1] & 0x3f) as u16);
            i += 2;
        } else if c & 0xf0 == 0xe0 && i + 2 < b.len() {
            out.push((((c & 0x0f) as u16) << 12) | (((b[i + 1] & 0x3f) as u16) << 6) | (b[i + 2] & 0x3f) as u16);
            i += 3;
        } else if c & 0xf8 == 0xf0 && i + 3 < b.len() {
            let cp = (((c & 7) as u32) << 18) | (((b[i + 1] & 0x3f) as u32) << 12) | (((b[i + 2] & 0x3f) as u32) << 6) | (b[i + 3] & 0x3f) as u32;
            let v = cp.wrapping_sub(0x10000);
            out.push(0xd800 + (v >> 10) as u16);
            out.push(0xdc00 + (v & 0x3ff) as u16);
            i += 4;
        } else {
            out.push(c as u16);
            i += 1;
        }
    }
    out
}

fn mutf8_encode(u: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(u.len());
    for &c in u {
        if c == 0 {
            out.push(0xc0);
            out.push(0x80);
        } else if c < 0x80 {
            out.push(c as u8);
        } else if c < 0x800 {
            out.push(0xc0 | (c >> 6) as u8);
            out.push(0x80 | (c & 0x3f) as u8);
        } else {
            out.push(0xe0 | (c >> 12) as u8);
            out.push(0x80 | ((c >> 6) & 0x3f) as u8);
            out.push(0x80 | (c & 0x3f) as u8);
        }
    }
    out
}

#[repr(C)]
struct Blk {
    gp: [u64; 6],
    fp: [u64; 8],
    nstack: u64,
    stack: *const u64,
    target: u64,
    out: [u64; 2],
}

#[derive(Clone, Copy)]
pub enum A {
    I(u64),
    F(u64),
}

#[inline(never)]
fn probe_stack() {
    let marker = 0u8;
    let sp = &marker as *const u8 as usize;
    for k in 1..=64usize {
        unsafe {
            std::ptr::read_volatile((sp - k * 4096) as *const u8);
        }
    }
}

pub unsafe fn dyn_call(target: usize, args: &[A]) -> (u64, u64) {
    let mut blk = Blk { gp: [0; 6], fp: [0; 8], nstack: 0, stack: std::ptr::null(), target: target as u64, out: [0; 2] };
    let (mut ng, mut nf) = (0usize, 0usize);
    let mut st: Vec<u64> = Vec::new();
    for a in args {
        match *a {
            A::I(v) => {
                if ng < 6 {
                    blk.gp[ng] = v;
                    ng += 1;
                } else {
                    st.push(v);
                }
            }
            A::F(v) => {
                if nf < 8 {
                    blk.fp[nf] = v;
                    nf += 1;
                } else {
                    st.push(v);
                }
            }
        }
    }
    blk.nstack = st.len() as u64;
    blk.stack = st.as_ptr();
    probe_stack();
    let p = &mut blk as *mut Blk;
    std::arch::asm!(
        "mov r12, rsp",
        "and rsp, -16",
        "mov rcx, qword ptr [r13 + 112]",
        "mov rdx, qword ptr [r13 + 120]",
        "test cl, 1",
        "jz 2f",
        "sub rsp, 8",
        "2:",
        "test rcx, rcx",
        "jz 3f",
        "dec rcx",
        "push qword ptr [rdx + rcx*8]",
        "jmp 2b",
        "3:",
        "movsd xmm0, qword ptr [r13 + 48]",
        "movsd xmm1, qword ptr [r13 + 56]",
        "movsd xmm2, qword ptr [r13 + 64]",
        "movsd xmm3, qword ptr [r13 + 72]",
        "movsd xmm4, qword ptr [r13 + 80]",
        "movsd xmm5, qword ptr [r13 + 88]",
        "movsd xmm6, qword ptr [r13 + 96]",
        "movsd xmm7, qword ptr [r13 + 104]",
        "mov rdi, qword ptr [r13]",
        "mov rsi, qword ptr [r13 + 8]",
        "mov rdx, qword ptr [r13 + 16]",
        "mov rcx, qword ptr [r13 + 24]",
        "mov r8, qword ptr [r13 + 32]",
        "mov r9, qword ptr [r13 + 40]",
        "mov r11, qword ptr [r13 + 128]",
        "mov eax, 8",
        "call r11",
        "mov qword ptr [r13 + 136], rax",
        "movsd qword ptr [r13 + 144], xmm0",
        "mov rsp, r12",
        in("r13") p,
        out("r12") _,
        clobber_abi("sysv64"),
    );
    drop(st);
    (blk.out[0], blk.out[1])
}

fn env_ptr() -> usize {
    *ENV.get_or_init(|| {
        let table: &'static [usize] = Box::leak(build_table().into_boxed_slice());
        let cell: &'static usize = Box::leak(Box::new(table.as_ptr() as usize));
        cell as *const usize as usize
    })
}

fn vm_ptr() -> usize {
    *VM.get_or_init(|| {
        let table: &'static [usize; 8] = Box::leak(Box::new([0, 0, 0, vm_destroy as usize, vm_attach as usize, vm_detach as usize, vm_get_env as usize, vm_attach as usize]));
        let cell: &'static usize = Box::leak(Box::new(table.as_ptr() as usize));
        cell as *const usize as usize
    })
}

unsafe extern "sysv64" fn vm_destroy(_vm: usize) -> i32 {
    0
}

unsafe extern "sysv64" fn vm_attach(_vm: usize, penv: *mut usize, _args: usize) -> i32 {
    *penv = env_ptr();
    0
}

unsafe extern "sysv64" fn vm_detach(_vm: usize) -> i32 {
    0
}

unsafe extern "sysv64" fn vm_get_env(_vm: usize, penv: *mut usize, _ver: i32) -> i32 {
    *penv = env_ptr();
    0
}

fn mangle(s: &str) -> String {
    let mut o = String::new();
    for ch in s.chars() {
        match ch {
            '/' | '.' => o.push('_'),
            '_' => o.push_str("_1"),
            ';' => o.push_str("_2"),
            '[' => o.push_str("_3"),
            c if c.is_ascii_alphanumeric() => o.push(c),
            c => {
                let mut b = [0u16; 2];
                for u in c.encode_utf16(&mut b) {
                    o.push_str(&format!("_0{:04x}", u));
                }
            }
        }
    }
    o
}

fn jni_names(class_desc: &str, name: &str, key: &str) -> (String, String) {
    let cn = if class_desc.len() > 2 { &class_desc[1..class_desc.len() - 1] } else { class_desc };
    let short = format!("Java_{}_{}", mangle(cn), mangle(name));
    let args = match (key.find('('), key.find(')')) {
        (Some(a), Some(b)) if b > a => &key[a + 1..b],
        _ => "",
    };
    let long = format!("{}__{}", short, mangle(args));
    (short, long)
}

fn jni_trace() -> bool {
    static T: OnceLock<bool> = OnceLock::new();
    *T.get_or_init(|| std::env::var("WINART_JNITRACE").is_ok())
}

impl Runtime {
    pub(crate) fn jni_invoke(&mut self, m: &Rc<Method>, args: &[u32]) -> Option<R<u64>> {
        let app = self.jni.app_dex.map(|d| m.dex >= d).unwrap_or(false);
        if !app {
            return None;
        }
        let ck = (m.class, m.key.clone());
        let found = self.jni.registered.get(&ck).or_else(|| self.jni.bound.get(&ck)).copied();
        let addr = match found {
            Some(a) => a,
            None => {
                let cn = self.classes[m.class].name.clone();
                let (short, long) = jni_names(&cn, &m.name, &m.key);
                match elf::find_symbol(&short).or_else(|| elf::find_symbol(&long)) {
                    Some(a) => {
                        self.jni.bound.insert(ck, a);
                        a
                    }
                    None => {
                        let msg = format!("No implementation found for {}.{} (tried {} and {})", cn, m.key, short, long);
                        return Some(Err(self.throw_msg("Ljava/lang/UnsatisfiedLinkError;", &msg)));
                    }
                }
            }
        };
        Some(self.jni_call(m, addr, args))
    }

    fn jni_call(&mut self, m: &Rc<Method>, addr: usize, args: &[u32]) -> R<u64> {
        RT.store(self as *mut Runtime, Ordering::Relaxed);
        let is_static = m.access & 8 != 0;
        let (params, ret) = parse_sig(&m.key);
        let t = tid();
        let mut a: Vec<A> = vec![A::I(env_ptr() as u64)];
        let mut idx = 0usize;
        if is_static {
            let co = self.class_obj(m.class);
            self.jni.locals.push((t, co));
            a.push(A::I(co as u64));
        } else {
            let o = args.first().copied().unwrap_or(0);
            if o != 0 {
                self.jni.locals.push((t, o));
            }
            a.push(A::I(o as u64));
            idx = 1;
        }
        for &p in &params {
            let g = |i: usize| args.get(i).copied().unwrap_or(0);
            match p {
                b'Z' => {
                    a.push(A::I((g(idx) & 0xff != 0) as u64));
                    idx += 1;
                }
                b'B' => {
                    a.push(A::I(g(idx) as i8 as i64 as u64));
                    idx += 1;
                }
                b'C' => {
                    a.push(A::I(g(idx) as u16 as u64));
                    idx += 1;
                }
                b'S' => {
                    a.push(A::I(g(idx) as i16 as i64 as u64));
                    idx += 1;
                }
                b'I' => {
                    a.push(A::I(g(idx) as i32 as i64 as u64));
                    idx += 1;
                }
                b'J' => {
                    a.push(A::I(g(idx) as u64 | ((g(idx + 1) as u64) << 32)));
                    idx += 2;
                }
                b'F' => {
                    a.push(A::F(g(idx) as u64));
                    idx += 1;
                }
                b'D' => {
                    a.push(A::F(g(idx) as u64 | ((g(idx + 1) as u64) << 32)));
                    idx += 2;
                }
                _ => {
                    let r = g(idx);
                    if r != 0 {
                        self.jni.locals.push((t, r));
                    }
                    a.push(A::I(r as u64));
                    idx += 1;
                }
            }
        }
        if jni_trace() {
            eprintln!("[jni] call {}.{} @ {:#x}", self.classes[m.class].name, m.key, addr);
        }
        NDEPTH.with(|d| d.set(d.get() + 1));
        let (rax, xmm) = unsafe { dyn_call(addr, &a) };
        let depth = NDEPTH.with(|d| {
            d.set(d.get() - 1);
            d.get()
        });
        let ex = take_pending();
        if depth == 0 {
            rt().jni.locals.retain(|(tt, _)| *tt != t);
        }
        if ex != 0 {
            return Err(ex);
        }
        Ok(match ret {
            b'V' => 0,
            b'Z' => (rax & 0xff != 0) as u64,
            b'B' => (rax as i8 as i32 as u32) as u64,
            b'C' => (rax as u16) as u64,
            b'S' => (rax as i16 as i32 as u32) as u64,
            b'I' => (rax as u32) as u64,
            b'J' => rax,
            b'F' => (xmm as u32) as u64,
            b'D' => xmm,
            _ => rax as u32 as u64,
        })
    }

    fn apk_lib_bytes(&self, file: &str) -> Result<Vec<u8>, String> {
        let z = match &self.apk {
            Some(z) => z,
            None => return Err(format!("dlopen failed: library \"{}\" not found", file)),
        };
        let want = format!("lib/x86_64/{}", file);
        if let Some(b) = z.read(&want) {
            return Ok(b);
        }
        let suffix = format!("/{}", file);
        let mut abis: Vec<String> = Vec::new();
        for e in &z.entries {
            if e.name.starts_with("lib/") && e.name.ends_with(&suffix) {
                let abi = e.name["lib/".len()..e.name.len() - suffix.len()].to_string();
                if !abis.contains(&abi) {
                    abis.push(abi);
                }
            }
        }
        if abis.is_empty() {
            Err(format!("dlopen failed: library \"{}\" not found", file))
        } else {
            Err(format!("dlopen failed: library \"{}\" is only available for ABI {} but winart runs x86_64 code only", file, abis.join(", ")))
        }
    }

    pub(crate) fn jni_load_library(&mut self, name: &str) -> Result<(), LoadErr> {
        let file = format!("lib{}.so", name);
        self.jni_load_file(&file, None)
    }

    pub(crate) fn jni_load_path(&mut self, path: &str) -> Result<(), LoadErr> {
        let file = path.rsplit(['/', '\\']).next().unwrap_or(path).to_string();
        if elf::is_loaded(&file) {
            return Ok(());
        }
        match std::fs::read(path) {
            Ok(b) => self.jni_load_file(&file, Some(b)),
            Err(_) => self.jni_load_file(&file, None),
        }
    }

    fn jni_load_file(&mut self, file: &str, direct: Option<Vec<u8>>) -> Result<(), LoadErr> {
        RT.store(self as *mut Runtime, Ordering::Relaxed);
        if elf::is_loaded(file) {
            return Ok(());
        }
        let bytes = match direct {
            Some(b) => b,
            None => self.apk_lib_bytes(file).map_err(LoadErr::Msg)?,
        };
        let arch = elf::arch_of(&bytes);
        if arch != "x86_64" {
            return Err(LoadErr::Msg(format!("dlopen failed: \"{}\" is {} code but winart runs x86_64 code only", file, arch)));
        }
        let needed = elf::needed(&bytes).map_err(|e| LoadErr::Msg(format!("dlopen failed: {}: {}", file, e)))?;
        for n in needed {
            if crate::bionic::is_system_lib(&n) || elf::is_loaded(&n) {
                continue;
            }
            self.jni_load_file(&n, None).map_err(|e| match e {
                LoadErr::Msg(m) => LoadErr::Msg(format!("{} (needed by {})", m, file)),
                x => x,
            })?;
        }
        let mut resolve = |s: &str| crate::bionic::lookup(s).or_else(|| elf::find_symbol(s));
        let lib = elf::load(file, &bytes, &mut resolve).map_err(|e| LoadErr::Msg(format!("dlopen failed: {}: {}", file, e)))?;
        if !lib.unresolved.is_empty() {
            let mut u = lib.unresolved.clone();
            u.sort();
            u.dedup();
            let shown: Vec<String> = u.iter().take(16).cloned().collect();
            eprintln!("winart: {}: {} unresolved imports (calling one aborts): {}{}", file, u.len(), shown.join(", "), if u.len() > 16 { ", ..." } else { "" });
        }
        elf::register(lib);
        self.jni.libs.push(file.to_string());
        self.jni_on_load(file)
    }

    fn jni_on_load(&mut self, file: &str) -> Result<(), LoadErr> {
        let addr = {
            let g = elf::LIBS.lock().unwrap_or_else(|e| e.into_inner());
            g.iter().find(|l| l.name == file).and_then(|l| l.syms.get("JNI_OnLoad").copied())
        };
        let addr = match addr {
            Some(a) => a,
            None => return Ok(()),
        };
        let t = tid();
        NDEPTH.with(|d| d.set(d.get() + 1));
        let (rax, _) = unsafe { dyn_call(addr, &[A::I(vm_ptr() as u64), A::I(0)]) };
        let depth = NDEPTH.with(|d| {
            d.set(d.get() - 1);
            d.get()
        });
        let ex = take_pending();
        if depth == 0 {
            rt().jni.locals.retain(|(tt, _)| *tt != t);
        }
        if ex != 0 {
            return Err(LoadErr::Exc(ex));
        }
        let v = rax as u32 as i32;
        if !matches!(v, 0x10001 | 0x10002 | 0x10004 | 0x10006 | 0x10008) {
            return Err(LoadErr::Msg(format!("JNI_OnLoad in {} returned bad version {:#x}", file, v)));
        }
        Ok(())
    }

    pub(crate) fn native_load_hook(&mut self, key: &str, a: &[u32]) -> Option<R<u64>> {
        let (is_path, arg) = match key {
            "loadLibrary(Ljava/lang/String;)V" | "loadLibrary0(Ljava/lang/Class;Ljava/lang/String;)V" | "loadLibrary0(Ljava/lang/ClassLoader;Ljava/lang/String;)V" => (false, a.last().copied()?),
            "load(Ljava/lang/String;)V" | "load0(Ljava/lang/Class;Ljava/lang/String;)V" => (true, a.last().copied()?),
            "mapLibraryName(Ljava/lang/String;)Ljava/lang/String;" => {
                let n = self.string_value(*a.first()?);
                return Some(Ok(self.str_ref(&format!("lib{}.so", n)) as u64));
            }
            _ => return None,
        };
        if arg == 0 {
            return Some(Err(self.npe()));
        }
        let n = self.string_value(arg);
        let r = if is_path { self.jni_load_path(&n) } else { self.jni_load_library(&n) };
        Some(match r {
            Ok(()) => Ok(0),
            Err(LoadErr::Exc(e)) => Err(e),
            Err(LoadErr::Msg(m)) => {
                let m = if is_path { m } else { format!("{}; no \"{}\" in java.library.path", m, n) };
                Err(self.throw_msg("Ljava/lang/UnsatisfiedLinkError;", &m))
            }
        })
    }
}

macro_rules! xthunk {
    ($name:ident, $target:path) => {
        std::arch::global_asm!(
            ".text",
            ".p2align 4",
            concat!(".globl ", stringify!($name)),
            concat!(stringify!($name), ":"),
            "sub rsp, 8",
            "call {t}",
            "movq xmm0, rax",
            "add rsp, 8",
            "ret",
            t = sym $target,
        );
        extern "sysv64" {
            fn $name();
        }
    };
}

macro_rules! fpthunk {
    ($name:ident, $target:path) => {
        std::arch::global_asm!(
            ".text",
            ".p2align 4",
            concat!(".globl ", stringify!($name)),
            concat!(stringify!($name), ":"),
            "movq rcx, xmm0",
            "jmp {t}",
            t = sym $target,
        );
        extern "sysv64" {
            fn $name();
        }
    };
}

vstub!(winart_jni_call_inst, 24, "rcx", call_inst_v);
vstub!(winart_jni_call_nv, 32, "r8", call_nv_v);
vstub!(winart_jni_call_st, 24, "rcx", call_st_v);
vstub!(winart_jni_newobject, 24, "rcx", new_object_v);
xthunk!(winart_jni_call_inst_v, call_inst_v);
xthunk!(winart_jni_call_inst_a, call_inst_a);
xthunk!(winart_jni_call_nv_v, call_nv_v);
xthunk!(winart_jni_call_nv_a, call_nv_a);
xthunk!(winart_jni_call_st_v, call_st_v);
xthunk!(winart_jni_call_st_a, call_st_a);
xthunk!(winart_jni_get_field, get_field_raw);
xthunk!(winart_jni_get_static_field, get_static_field_raw);
fpthunk!(winart_jni_set_field_fp, set_field_raw);
fpthunk!(winart_jni_set_static_field_fp, set_static_field_raw);

const T10: [&str; 10] = ["Object", "Boolean", "Byte", "Char", "Short", "Int", "Long", "Float", "Double", "Void"];
const P8: [&str; 8] = ["Boolean", "Byte", "Char", "Short", "Int", "Long", "Float", "Double"];

fn table_names() -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for i in 0..4 {
        v.push(format!("reserved{}", i));
    }
    for n in [
        "GetVersion",
        "DefineClass",
        "FindClass",
        "FromReflectedMethod",
        "FromReflectedField",
        "ToReflectedMethod",
        "GetSuperclass",
        "IsAssignableFrom",
        "ToReflectedField",
        "Throw",
        "ThrowNew",
        "ExceptionOccurred",
        "ExceptionDescribe",
        "ExceptionClear",
        "FatalError",
        "PushLocalFrame",
        "PopLocalFrame",
        "NewGlobalRef",
        "DeleteGlobalRef",
        "DeleteLocalRef",
        "IsSameObject",
        "NewLocalRef",
        "EnsureLocalCapacity",
        "AllocObject",
        "NewObject",
        "NewObjectV",
        "NewObjectA",
        "GetObjectClass",
        "IsInstanceOf",
        "GetMethodID",
    ] {
        v.push(n.to_string());
    }
    for t in T10 {
        for s in ["", "V", "A"] {
            v.push(format!("Call{}Method{}", t, s));
        }
    }
    for t in T10 {
        for s in ["", "V", "A"] {
            v.push(format!("CallNonvirtual{}Method{}", t, s));
        }
    }
    v.push("GetFieldID".to_string());
    for t in &T10[..9] {
        v.push(format!("Get{}Field", t));
    }
    for t in &T10[..9] {
        v.push(format!("Set{}Field", t));
    }
    v.push("GetStaticMethodID".to_string());
    for t in T10 {
        for s in ["", "V", "A"] {
            v.push(format!("CallStatic{}Method{}", t, s));
        }
    }
    v.push("GetStaticFieldID".to_string());
    for t in &T10[..9] {
        v.push(format!("GetStatic{}Field", t));
    }
    for t in &T10[..9] {
        v.push(format!("SetStatic{}Field", t));
    }
    for n in [
        "NewString",
        "GetStringLength",
        "GetStringChars",
        "ReleaseStringChars",
        "NewStringUTF",
        "GetStringUTFLength",
        "GetStringUTFChars",
        "ReleaseStringUTFChars",
        "GetArrayLength",
        "NewObjectArray",
        "GetObjectArrayElement",
        "SetObjectArrayElement",
    ] {
        v.push(n.to_string());
    }
    for fmt in ["New{}Array", "Get{}ArrayElements", "Release{}ArrayElements", "Get{}ArrayRegion", "Set{}ArrayRegion"] {
        for t in P8 {
            v.push(fmt.replace("{}", t));
        }
    }
    for n in [
        "RegisterNatives",
        "UnregisterNatives",
        "MonitorEnter",
        "MonitorExit",
        "GetJavaVM",
        "GetStringRegion",
        "GetStringUTFRegion",
        "GetPrimitiveArrayCritical",
        "ReleasePrimitiveArrayCritical",
        "GetStringCritical",
        "ReleaseStringCritical",
        "NewWeakGlobalRef",
        "DeleteWeakGlobalRef",
        "ExceptionCheck",
        "NewDirectByteBuffer",
        "GetDirectBufferAddress",
        "GetDirectBufferCapacity",
        "GetObjectRefType",
        "GetModule",
        "IsVirtualThread",
    ] {
        v.push(n.to_string());
    }
    v
}

fn build_table() -> Vec<usize> {
    let names = table_names();
    let mut t: Vec<usize> = names.iter().map(|n| elf::stub_for(&format!("JNIEnv::{}", n))).collect();
    let mut put = |n: &str, f: usize| {
        if let Some(i) = names.iter().position(|x| x == n) {
            t[i] = f;
        }
    };
    put("GetVersion", get_version as usize);
    put("DefineClass", define_class as usize);
    put("FindClass", find_class as usize);
    put("GetSuperclass", get_superclass as usize);
    put("IsAssignableFrom", is_assignable_from as usize);
    put("Throw", throw_obj as usize);
    put("ThrowNew", throw_new_fn as usize);
    put("ExceptionOccurred", exception_occurred as usize);
    put("ExceptionDescribe", exception_describe as usize);
    put("ExceptionClear", exception_clear as usize);
    put("FatalError", fatal_error as usize);
    put("PushLocalFrame", push_local_frame as usize);
    put("PopLocalFrame", pop_local_frame as usize);
    put("NewGlobalRef", new_global_ref as usize);
    put("DeleteGlobalRef", delete_global_ref as usize);
    put("DeleteLocalRef", delete_local_ref as usize);
    put("IsSameObject", is_same_object as usize);
    put("NewLocalRef", new_local_ref as usize);
    put("EnsureLocalCapacity", ensure_local_capacity as usize);
    put("AllocObject", alloc_object as usize);
    put("NewObject", winart_jni_newobject as usize);
    put("NewObjectV", new_object_v as usize);
    put("NewObjectA", new_object_a as usize);
    put("GetObjectClass", get_object_class as usize);
    put("IsInstanceOf", is_instance_of as usize);
    put("GetMethodID", get_method_id as usize);
    put("GetFieldID", get_field_id as usize);
    put("GetStaticMethodID", get_static_method_id as usize);
    put("GetStaticFieldID", get_static_field_id as usize);
    for ty in T10 {
        put(&format!("Call{}Method", ty), winart_jni_call_inst as usize);
        put(&format!("Call{}MethodV", ty), winart_jni_call_inst_v as usize);
        put(&format!("Call{}MethodA", ty), winart_jni_call_inst_a as usize);
        put(&format!("CallNonvirtual{}Method", ty), winart_jni_call_nv as usize);
        put(&format!("CallNonvirtual{}MethodV", ty), winart_jni_call_nv_v as usize);
        put(&format!("CallNonvirtual{}MethodA", ty), winart_jni_call_nv_a as usize);
        put(&format!("CallStatic{}Method", ty), winart_jni_call_st as usize);
        put(&format!("CallStatic{}MethodV", ty), winart_jni_call_st_v as usize);
        put(&format!("CallStatic{}MethodA", ty), winart_jni_call_st_a as usize);
    }
    for ty in &T10[..9] {
        let fp = *ty == "Float" || *ty == "Double";
        put(&format!("Get{}Field", ty), winart_jni_get_field as usize);
        put(&format!("GetStatic{}Field", ty), winart_jni_get_static_field as usize);
        put(&format!("Set{}Field", ty), if fp { winart_jni_set_field_fp as usize } else { set_field_raw as usize });
        put(&format!("SetStatic{}Field", ty), if fp { winart_jni_set_static_field_fp as usize } else { set_static_field_raw as usize });
    }
    put("NewString", new_string as usize);
    put("GetStringLength", get_string_length as usize);
    put("GetStringChars", get_string_chars as usize);
    put("ReleaseStringChars", release_string_chars as usize);
    put("NewStringUTF", new_string_utf as usize);
    put("GetStringUTFLength", get_string_utf_length as usize);
    put("GetStringUTFChars", get_string_utf_chars as usize);
    put("ReleaseStringUTFChars", release_string_chars as usize);
    put("GetArrayLength", get_array_length as usize);
    put("NewObjectArray", new_object_array as usize);
    put("GetObjectArrayElement", get_object_array_element as usize);
    put("SetObjectArrayElement", set_object_array_element as usize);
    let news: [usize; 8] = [
        new_prim_array::<{ b'Z' }> as usize,
        new_prim_array::<{ b'B' }> as usize,
        new_prim_array::<{ b'C' }> as usize,
        new_prim_array::<{ b'S' }> as usize,
        new_prim_array::<{ b'I' }> as usize,
        new_prim_array::<{ b'J' }> as usize,
        new_prim_array::<{ b'F' }> as usize,
        new_prim_array::<{ b'D' }> as usize,
    ];
    for (i, ty) in P8.iter().enumerate() {
        put(&format!("New{}Array", ty), news[i]);
        put(&format!("Get{}ArrayElements", ty), get_arr_elems as usize);
        put(&format!("Release{}ArrayElements", ty), release_arr_elems as usize);
        put(&format!("Get{}ArrayRegion", ty), get_arr_region as usize);
        put(&format!("Set{}ArrayRegion", ty), set_arr_region as usize);
    }
    put("RegisterNatives", register_natives as usize);
    put("UnregisterNatives", unregister_natives as usize);
    put("MonitorEnter", monitor_enter_fn as usize);
    put("MonitorExit", monitor_exit_fn as usize);
    put("GetJavaVM", get_java_vm as usize);
    put("GetStringRegion", get_string_region as usize);
    put("GetStringUTFRegion", get_string_utf_region as usize);
    put("GetPrimitiveArrayCritical", get_arr_elems as usize);
    put("ReleasePrimitiveArrayCritical", release_arr_elems as usize);
    put("GetStringCritical", get_string_chars as usize);
    put("ReleaseStringCritical", release_string_chars as usize);
    put("NewWeakGlobalRef", new_global_ref as usize);
    put("DeleteWeakGlobalRef", delete_global_ref as usize);
    put("ExceptionCheck", exception_check as usize);
    put("NewDirectByteBuffer", new_direct_buffer as usize);
    put("GetDirectBufferAddress", get_direct_address as usize);
    put("GetDirectBufferCapacity", get_direct_capacity as usize);
    put("GetObjectRefType", get_ref_type as usize);
    put("GetModule", get_module as usize);
    put("IsVirtualThread", is_virtual_thread as usize);
    t
}

fn ret_val(ret: u8, v: u64) -> u64 {
    match ret {
        b'V' => 0,
        b'Z' => v & 1,
        b'B' => v as i8 as i64 as u64,
        b'C' => v as u16 as u64,
        b'S' => v as i16 as i64 as u64,
        b'I' => v as i32 as i64 as u64,
        b'J' | b'D' => v,
        b'F' => v as u32 as u64,
        _ => loc(v as u32 as Ref) as u64,
    }
}

fn field_in(kind: u8, v: u64) -> u64 {
    match kind {
        b'Z' => (v & 0xff != 0) as u64,
        b'B' => v as i8 as i32 as u32 as u64,
        b'C' => v as u16 as u64,
        b'S' => v as i16 as i32 as u32 as u64,
        b'J' | b'D' => v,
        _ => v as u32 as u64,
    }
}

fn norm_arg(p: u8, v: u64) -> u32 {
    match p {
        b'Z' => (v & 0xff != 0) as u32,
        b'B' => v as i8 as i32 as u32,
        b'C' => v as u16 as u32,
        b'S' => v as i16 as i32 as u32,
        _ => v as u32,
    }
}

unsafe fn args_va(params: &[u8], ap: *mut VaList) -> Vec<u32> {
    let ap = &mut *ap;
    let mut out = Vec::with_capacity(params.len() + 2);
    for &p in params {
        match p {
            b'J' => {
                let v = ap.gp();
                out.push(v as u32);
                out.push((v >> 32) as u32);
            }
            b'D' => {
                let v = ap.fp().to_bits();
                out.push(v as u32);
                out.push((v >> 32) as u32);
            }
            b'F' => out.push((ap.fp() as f32).to_bits()),
            _ => out.push(norm_arg(p, ap.gp())),
        }
    }
    out
}

unsafe fn args_a(params: &[u8], a: *const u64) -> Vec<u32> {
    let mut out = Vec::with_capacity(params.len() + 2);
    for (i, &p) in params.iter().enumerate() {
        let v = *a.add(i);
        match p {
            b'J' | b'D' => {
                out.push(v as u32);
                out.push((v >> 32) as u32);
            }
            _ => out.push(norm_arg(p, v)),
        }
    }
    out
}

fn mparams(mid: usize) -> Vec<u8> {
    rt().jni.methods.get(mid.wrapping_sub(1)).map(|m| m.params.clone()).unwrap_or_default()
}

fn call_core(mode: u8, recv: usize, cls: usize, mid: usize, args: Vec<u32>) -> u64 {
    let r = rt();
    let (mcls, key, ret, st) = match r.jni.methods.get(mid.wrapping_sub(1)) {
        Some(m) => (m.class, m.key.clone(), m.ret, m.is_static),
        None => {
            throw("Ljava/lang/NoSuchMethodError;", "invalid jmethodID");
            return 0;
        }
    };
    let res = if st {
        let c = if cls != 0 { class_idx(cls) } else { mcls };
        match r.ensure_init(c) {
            Err(e) => Err(e),
            Ok(()) => match r.find_method(c, &key) {
                Some(m) => r.invoke(&m, args),
                None => Err(r.throw_new("Ljava/lang/NoSuchMethodError;")),
            },
        }
    } else if recv == 0 {
        Err(r.npe())
    } else if mode == 1 {
        let c = if cls != 0 { class_idx(cls) } else { mcls };
        match r.find_method(c, &key) {
            Some(m) => {
                let mut a = vec![recv as u32];
                a.extend(args);
                r.invoke(&m, a)
            }
            None => Err(r.throw_new("Ljava/lang/NoSuchMethodError;")),
        }
    } else {
        r.call_virtual(recv as Ref, &key, args)
    };
    match res {
        Ok(v) => ret_val(ret, v),
        Err(e) => {
            set_pending(e);
            0
        }
    }
}

unsafe extern "sysv64" fn call_inst_v(_e: usize, obj: usize, mid: usize, ap: *mut VaList) -> u64 {
    let p = mparams(mid);
    call_core(0, obj, 0, mid, args_va(&p, ap))
}

unsafe extern "sysv64" fn call_inst_a(_e: usize, obj: usize, mid: usize, a: *const u64) -> u64 {
    let p = mparams(mid);
    call_core(0, obj, 0, mid, args_a(&p, a))
}

unsafe extern "sysv64" fn call_nv_v(_e: usize, obj: usize, cls: usize, mid: usize, ap: *mut VaList) -> u64 {
    let p = mparams(mid);
    call_core(1, obj, cls, mid, args_va(&p, ap))
}

unsafe extern "sysv64" fn call_nv_a(_e: usize, obj: usize, cls: usize, mid: usize, a: *const u64) -> u64 {
    let p = mparams(mid);
    call_core(1, obj, cls, mid, args_a(&p, a))
}

unsafe extern "sysv64" fn call_st_v(_e: usize, cls: usize, mid: usize, ap: *mut VaList) -> u64 {
    let p = mparams(mid);
    call_core(2, 0, cls, mid, args_va(&p, ap))
}

unsafe extern "sysv64" fn call_st_a(_e: usize, cls: usize, mid: usize, a: *const u64) -> u64 {
    let p = mparams(mid);
    call_core(2, 0, cls, mid, args_a(&p, a))
}

fn new_object_core(cls: usize, mid: usize, args: Vec<u32>) -> usize {
    let r = rt();
    let c = class_idx(cls);
    let key = match r.jni.methods.get(mid.wrapping_sub(1)) {
        Some(m) => m.key.clone(),
        None => {
            throw("Ljava/lang/NoSuchMethodError;", "invalid jmethodID");
            return 0;
        }
    };
    if let Err(e) = r.ensure_init(c) {
        set_pending(e);
        return 0;
    }
    if r.classes[c].name == "Ljava/lang/String;" {
        let mut a = vec![0u32];
        a.extend(args);
        return match r.string_init(&key, a) {
            Ok(s) => loc(s),
            Err(e) => {
                set_pending(e);
                0
            }
        };
    }
    let n = r.classes[c].inst_total;
    let o = r.alloc(Obj { class: c, fields: vec![0; n], kind: Kind::Plain });
    loc(o);
    let m = match r.find_method(c, &key) {
        Some(m) => m,
        None => {
            throw("Ljava/lang/NoSuchMethodError;", &key);
            return 0;
        }
    };
    let mut a = vec![o];
    a.extend(args);
    match r.invoke(&m, a) {
        Ok(_) => o as usize,
        Err(e) => {
            set_pending(e);
            0
        }
    }
}

unsafe extern "sysv64" fn new_object_v(_e: usize, cls: usize, mid: usize, ap: *mut VaList) -> usize {
    let p = mparams(mid);
    new_object_core(cls, mid, args_va(&p, ap))
}

unsafe extern "sysv64" fn new_object_a(_e: usize, cls: usize, mid: usize, a: *const u64) -> usize {
    let p = mparams(mid);
    new_object_core(cls, mid, args_a(&p, a))
}

unsafe extern "sysv64" fn get_version(_e: usize) -> i32 {
    0x10006
}

unsafe extern "sysv64" fn define_class(_e: usize, _n: *const u8, _l: usize, _b: *const u8, _len: i32) -> usize {
    throw("Ljava/lang/UnsupportedOperationException;", "DefineClass");
    0
}

unsafe extern "sysv64" fn find_class(_e: usize, name: *const u8) -> usize {
    let n = cstring(name).replace('.', "/");
    let desc = if n.starts_with('[') { n.clone() } else { format!("L{};", n) };
    let r = rt();
    if !desc.starts_with('[') && !r.defs.contains_key(&desc) {
        throw("Ljava/lang/NoClassDefFoundError;", &n);
        return 0;
    }
    let c = r.load(&desc);
    if let Err(e) = r.ensure_init(c) {
        set_pending(e);
        return 0;
    }
    loc(r.class_obj(c))
}

unsafe extern "sysv64" fn get_superclass(_e: usize, cls: usize) -> usize {
    let r = rt();
    let c = class_idx(cls);
    match r.classes.get(c).and_then(|k| if k.access & 0x200 != 0 { None } else { k.sup }) {
        Some(s) => loc(r.class_obj(s)),
        None => 0,
    }
}

unsafe extern "sysv64" fn is_assignable_from(_e: usize, sub: usize, sup: usize) -> u8 {
    rt().is_sub(class_idx(sub), class_idx(sup)) as u8
}

unsafe extern "sysv64" fn throw_obj(_e: usize, obj: usize) -> i32 {
    set_pending(obj as Ref);
    0
}

unsafe extern "sysv64" fn throw_new_fn(_e: usize, cls: usize, msg: *const u8) -> i32 {
    let name = rt().classes[class_idx(cls)].name.clone();
    let m = if msg.is_null() { String::new() } else { cstring(msg) };
    throw(&name, &m);
    0
}

unsafe extern "sysv64" fn exception_occurred(_e: usize) -> usize {
    PENDING.with(|p| p.get()) as usize
}

unsafe extern "sysv64" fn exception_describe(_e: usize) {
    let e = PENDING.with(|p| p.get());
    if e == 0 {
        return;
    }
    let r = rt();
    let c = r.heap[e as usize].class;
    let msg = r.classes[c].inst_idx.get("detailMessage").and_then(|&s| r.heap[e as usize].fields.get(s).copied()).filter(|&m| m != 0).map(|m| r.string_value(m as Ref));
    eprintln!("Exception in native code: {}: {}", r.classes[c].name, msg.unwrap_or_default());
}

unsafe extern "sysv64" fn exception_clear(_e: usize) {
    take_pending();
}

unsafe extern "sysv64" fn exception_check(_e: usize) -> u8 {
    (PENDING.with(|p| p.get()) != 0) as u8
}

unsafe extern "sysv64" fn fatal_error(_e: usize, msg: *const u8) {
    eprintln!("winart: JNI FatalError: {}", cstring(msg));
    std::process::exit(121);
}

unsafe extern "sysv64" fn push_local_frame(_e: usize, _cap: i32) -> i32 {
    0
}

unsafe extern "sysv64" fn pop_local_frame(_e: usize, res: usize) -> usize {
    res
}

unsafe extern "sysv64" fn ensure_local_capacity(_e: usize, _cap: i32) -> i32 {
    0
}

unsafe extern "sysv64" fn new_global_ref(_e: usize, obj: usize) -> usize {
    if obj != 0 {
        rt().jni.globals.push(obj as Ref);
    }
    obj
}

unsafe extern "sysv64" fn delete_global_ref(_e: usize, obj: usize) {
    let g = &mut rt().jni.globals;
    if let Some(i) = g.iter().rposition(|&x| x as usize == obj) {
        g.swap_remove(i);
    }
}

unsafe extern "sysv64" fn delete_local_ref(_e: usize, _obj: usize) {}

unsafe extern "sysv64" fn new_local_ref(_e: usize, obj: usize) -> usize {
    loc(obj as Ref)
}

unsafe extern "sysv64" fn is_same_object(_e: usize, a: usize, b: usize) -> u8 {
    (a == b) as u8
}

unsafe extern "sysv64" fn alloc_object(_e: usize, cls: usize) -> usize {
    let r = rt();
    let c = class_idx(cls);
    if let Err(e) = r.ensure_init(c) {
        set_pending(e);
        return 0;
    }
    let n = r.classes[c].inst_total;
    loc(r.alloc(Obj { class: c, fields: vec![0; n], kind: Kind::Plain }))
}

unsafe extern "sysv64" fn get_object_class(_e: usize, obj: usize) -> usize {
    let r = rt();
    match r.heap.get(obj) {
        Some(o) if obj != 0 => {
            let c = o.class;
            loc(r.class_obj(c))
        }
        _ => 0,
    }
}

unsafe extern "sysv64" fn is_instance_of(_e: usize, obj: usize, cls: usize) -> u8 {
    if obj == 0 {
        return 1;
    }
    let r = rt();
    match r.heap.get(obj) {
        Some(o) => r.is_sub(o.class, class_idx(cls)) as u8,
        None => 0,
    }
}

unsafe fn method_id(cls: usize, name: *const u8, sig: *const u8, st: bool) -> usize {
    let r = rt();
    let c = class_idx(cls);
    let key = format!("{}{}", cstring(name), cstring(sig));
    if let Some(&i) = r.jni.mcache.get(&(c, key.clone(), st)) {
        return i + 1;
    }
    if let Err(e) = r.ensure_init(c) {
        set_pending(e);
        return 0;
    }
    match r.find_method(c, &key) {
        Some(m) if (m.access & 8 != 0) == st => {
            let (params, ret) = parse_sig(&key);
            r.jni.methods.push(JMethod { class: c, key: key.clone(), params, ret, is_static: st });
            let i = r.jni.methods.len() - 1;
            r.jni.mcache.insert((c, key, st), i);
            i + 1
        }
        _ => {
            throw("Ljava/lang/NoSuchMethodError;", &key);
            0
        }
    }
}

unsafe extern "sysv64" fn get_method_id(_e: usize, cls: usize, name: *const u8, sig: *const u8) -> usize {
    method_id(cls, name, sig, false)
}

unsafe extern "sysv64" fn get_static_method_id(_e: usize, cls: usize, name: *const u8, sig: *const u8) -> usize {
    method_id(cls, name, sig, true)
}

unsafe fn field_id(cls: usize, name: *const u8, sig: *const u8, st: bool) -> usize {
    let r = rt();
    let c = class_idx(cls);
    let nm = cstring(name);
    let sg = cstring(sig);
    let kind = match sg.as_bytes().first() {
        Some(b'[' | b'L') | None => b'L',
        Some(&k) => k,
    };
    if let Some(&i) = r.jni.fcache.get(&(c, nm.clone(), st)) {
        return i + 1;
    }
    let found = if st {
        if let Err(e) = r.ensure_init(c) {
            set_pending(e);
            return 0;
        }
        r.find_static(c, &nm)
    } else {
        r.classes[c].inst_idx.get(&nm).map(|&s| (c, s))
    };
    match found {
        Some((ci, slot)) => {
            r.jni.fields.push(JField { class: ci, slot, kind, is_static: st });
            let i = r.jni.fields.len() - 1;
            r.jni.fcache.insert((c, nm, st), i);
            i + 1
        }
        None => {
            throw("Ljava/lang/NoSuchFieldError;", &nm);
            0
        }
    }
}

unsafe extern "sysv64" fn get_field_id(_e: usize, cls: usize, name: *const u8, sig: *const u8) -> usize {
    field_id(cls, name, sig, false)
}

unsafe extern "sysv64" fn get_static_field_id(_e: usize, cls: usize, name: *const u8, sig: *const u8) -> usize {
    field_id(cls, name, sig, true)
}

unsafe extern "sysv64" fn get_field_raw(_e: usize, obj: usize, fid: usize) -> u64 {
    let r = rt();
    let (slot, kind) = match r.jni.fields.get(fid.wrapping_sub(1)) {
        Some(f) => (f.slot, f.kind),
        None => return 0,
    };
    let v = r.heap.get(obj).and_then(|o| o.fields.get(slot)).copied().unwrap_or(0);
    ret_val(kind, v)
}

unsafe extern "sysv64" fn get_static_field_raw(_e: usize, _cls: usize, fid: usize) -> u64 {
    let r = rt();
    let (class, slot, kind) = match r.jni.fields.get(fid.wrapping_sub(1)) {
        Some(f) => (f.class, f.slot, f.kind),
        None => return 0,
    };
    let v = r.classes[class].statics.get(slot).copied().unwrap_or(0);
    ret_val(kind, v)
}

unsafe extern "sysv64" fn set_field_raw(_e: usize, obj: usize, fid: usize, v: u64) {
    let r = rt();
    let (slot, kind) = match r.jni.fields.get(fid.wrapping_sub(1)) {
        Some(f) => (f.slot, f.kind),
        None => return,
    };
    let sv = field_in(kind, v);
    if let Some(o) = r.heap.get_mut(obj) {
        if o.fields.len() <= slot {
            o.fields.resize(slot + 1, 0);
        }
        o.fields[slot] = sv;
    }
}

unsafe extern "sysv64" fn set_static_field_raw(_e: usize, _cls: usize, fid: usize, v: u64) {
    let r = rt();
    let (class, slot, kind) = match r.jni.fields.get(fid.wrapping_sub(1)) {
        Some(f) => (f.class, f.slot, f.kind),
        None => return,
    };
    let sv = field_in(kind, v);
    if let Some(s) = r.classes[class].statics.get_mut(slot) {
        *s = sv;
    }
}

fn str_units(s: usize) -> Option<Vec<u16>> {
    match rt().heap.get(s).map(|o| &o.kind) {
        Some(Kind::Str(u)) => Some(u.clone()),
        _ => None,
    }
}

unsafe extern "sysv64" fn new_string(_e: usize, chars: *const u16, len: i32) -> usize {
    let u: Vec<u16> = if chars.is_null() || len <= 0 { Vec::new() } else { std::slice::from_raw_parts(chars, len as usize).to_vec() };
    loc(rt().new_string(u))
}

unsafe extern "sysv64" fn get_string_length(_e: usize, s: usize) -> i32 {
    str_units(s).map(|u| u.len() as i32).unwrap_or(0)
}

unsafe extern "sysv64" fn get_string_chars(_e: usize, s: usize, is_copy: *mut u8) -> usize {
    let u = match str_units(s) {
        Some(u) => u,
        None => return 0,
    };
    let p = malloc((u.len() + 1) * 2) as *mut u16;
    if p.is_null() {
        return 0;
    }
    std::ptr::copy_nonoverlapping(u.as_ptr(), p, u.len());
    *p.add(u.len()) = 0;
    if !is_copy.is_null() {
        *is_copy = 1;
    }
    p as usize
}

unsafe extern "sysv64" fn release_string_chars(_e: usize, _s: usize, buf: *mut u8) {
    if !buf.is_null() {
        free(buf);
    }
}

unsafe extern "sysv64" fn new_string_utf(_e: usize, bytes: *const u8) -> usize {
    if bytes.is_null() {
        return 0;
    }
    loc(rt().new_string(mutf8_decode(cstr(bytes))))
}

unsafe extern "sysv64" fn get_string_utf_length(_e: usize, s: usize) -> i32 {
    str_units(s).map(|u| mutf8_encode(&u).len() as i32).unwrap_or(0)
}

unsafe extern "sysv64" fn get_string_utf_chars(_e: usize, s: usize, is_copy: *mut u8) -> usize {
    let u = match str_units(s) {
        Some(u) => u,
        None => return 0,
    };
    let enc = mutf8_encode(&u);
    let p = malloc(enc.len() + 1);
    if p.is_null() {
        return 0;
    }
    std::ptr::copy_nonoverlapping(enc.as_ptr(), p, enc.len());
    *p.add(enc.len()) = 0;
    if !is_copy.is_null() {
        *is_copy = 1;
    }
    p as usize
}

unsafe extern "sysv64" fn get_string_region(_e: usize, s: usize, start: i32, len: i32, buf: *mut u16) {
    let u = match str_units(s) {
        Some(u) => u,
        None => return,
    };
    if start < 0 || len < 0 || start as usize + len as usize > u.len() {
        throw("Ljava/lang/StringIndexOutOfBoundsException;", &format!("start {} len {} length {}", start, len, u.len()));
        return;
    }
    std::ptr::copy_nonoverlapping(u.as_ptr().add(start as usize), buf, len as usize);
}

unsafe extern "sysv64" fn get_string_utf_region(_e: usize, s: usize, start: i32, len: i32, buf: *mut u8) {
    let u = match str_units(s) {
        Some(u) => u,
        None => return,
    };
    if start < 0 || len < 0 || start as usize + len as usize > u.len() {
        throw("Ljava/lang/StringIndexOutOfBoundsException;", &format!("start {} len {} length {}", start, len, u.len()));
        return;
    }
    let enc = mutf8_encode(&u[start as usize..(start + len) as usize]);
    std::ptr::copy_nonoverlapping(enc.as_ptr(), buf, enc.len());
    *buf.add(enc.len()) = 0;
}

fn esize(k: u8) -> usize {
    match k {
        b'Z' | b'B' => 1,
        b'C' | b'S' => 2,
        b'I' | b'F' => 4,
        _ => 8,
    }
}

unsafe fn rd(p: *const u8, sz: usize) -> u64 {
    let mut v = 0u64;
    std::ptr::copy_nonoverlapping(p, &mut v as *mut u64 as *mut u8, sz);
    v
}

unsafe fn wr(p: *mut u8, sz: usize, v: u64) {
    std::ptr::copy_nonoverlapping(&v as *const u64 as *const u8, p, sz);
}

fn norm_elem(k: u8, v: u64) -> u64 {
    if k == b'Z' {
        (v & 0xff != 0) as u64
    } else {
        v
    }
}

fn arr_kind(arr: usize) -> u8 {
    let r = rt();
    match r.heap.get(arr) {
        Some(o) => r.classes[o.class].name.as_bytes().get(1).copied().unwrap_or(b'I'),
        None => b'I',
    }
}

unsafe extern "sysv64" fn get_array_length(_e: usize, arr: usize) -> i32 {
    match rt().heap.get(arr).map(|o| &o.kind) {
        Some(Kind::Array(v)) => v.len() as i32,
        _ => 0,
    }
}

unsafe extern "sysv64" fn new_object_array(_e: usize, len: i32, cls: usize, init: usize) -> usize {
    if len < 0 {
        throw("Ljava/lang/NegativeArraySizeException;", &len.to_string());
        return 0;
    }
    let r = rt();
    let desc = format!("[{}", r.classes[class_idx(cls)].name);
    loc(r.new_array(&desc, vec![init as u64; len as usize]))
}

unsafe extern "sysv64" fn get_object_array_element(_e: usize, arr: usize, i: i32) -> usize {
    let r = rt();
    let v = match r.heap.get(arr).map(|o| &o.kind) {
        Some(Kind::Array(v)) if i >= 0 && (i as usize) < v.len() => Some(v[i as usize]),
        _ => None,
    };
    match v {
        Some(x) => loc(x as Ref),
        None => {
            throw("Ljava/lang/ArrayIndexOutOfBoundsException;", &format!("index {}", i));
            0
        }
    }
}

unsafe extern "sysv64" fn set_object_array_element(_e: usize, arr: usize, i: i32, val: usize) {
    let r = rt();
    let mut ok = false;
    if let Some(o) = r.heap.get_mut(arr) {
        if let Kind::Array(v) = &mut o.kind {
            if i >= 0 && (i as usize) < v.len() {
                v[i as usize] = val as u64;
                ok = true;
            }
        }
    }
    if !ok {
        throw("Ljava/lang/ArrayIndexOutOfBoundsException;", &format!("index {}", i));
    }
}

unsafe extern "sysv64" fn new_prim_array<const K: u8>(_e: usize, len: i32) -> usize {
    if len < 0 {
        throw("Ljava/lang/NegativeArraySizeException;", &len.to_string());
        return 0;
    }
    let desc = format!("[{}", K as char);
    loc(rt().new_array(&desc, vec![0; len as usize]))
}

unsafe extern "sysv64" fn get_arr_elems(_e: usize, arr: usize, is_copy: *mut u8) -> usize {
    let r = rt();
    let vals = match r.heap.get(arr).map(|o| &o.kind) {
        Some(Kind::Array(v)) => v.clone(),
        _ => return 0,
    };
    let k = arr_kind(arr);
    let sz = esize(k);
    let p = malloc(vals.len() * sz + 8);
    if p.is_null() {
        return 0;
    }
    for (i, v) in vals.iter().enumerate() {
        wr(p.add(i * sz), sz, *v);
    }
    if !is_copy.is_null() {
        *is_copy = 1;
    }
    p as usize
}

unsafe extern "sysv64" fn release_arr_elems(_e: usize, arr: usize, buf: *mut u8, mode: i32) {
    if buf.is_null() {
        return;
    }
    let k = arr_kind(arr);
    let sz = esize(k);
    if mode != 2 {
        if let Some(o) = rt().heap.get_mut(arr) {
            if let Kind::Array(v) = &mut o.kind {
                for (i, slot) in v.iter_mut().enumerate() {
                    *slot = norm_elem(k, rd(buf.add(i * sz), sz));
                }
            }
        }
    }
    if mode != 1 {
        free(buf);
    }
}

unsafe extern "sysv64" fn get_arr_region(_e: usize, arr: usize, start: i32, len: i32, buf: *mut u8) {
    let k = arr_kind(arr);
    let sz = esize(k);
    let part: Option<Vec<u64>> = match rt().heap.get(arr).map(|o| &o.kind) {
        Some(Kind::Array(v)) if start >= 0 && len >= 0 && start as usize + len as usize <= v.len() => Some(v[start as usize..(start + len) as usize].to_vec()),
        _ => None,
    };
    match part {
        Some(p) => {
            for (i, v) in p.iter().enumerate() {
                wr(buf.add(i * sz), sz, *v);
            }
        }
        None => throw("Ljava/lang/ArrayIndexOutOfBoundsException;", &format!("start {} len {}", start, len)),
    }
}

unsafe extern "sysv64" fn set_arr_region(_e: usize, arr: usize, start: i32, len: i32, buf: *const u8) {
    let k = arr_kind(arr);
    let sz = esize(k);
    let mut ok = false;
    if let Some(o) = rt().heap.get_mut(arr) {
        if let Kind::Array(v) = &mut o.kind {
            if start >= 0 && len >= 0 && start as usize + len as usize <= v.len() {
                for i in 0..len as usize {
                    v[start as usize + i] = norm_elem(k, rd(buf.add(i * sz), sz));
                }
                ok = true;
            }
        }
    }
    if !ok {
        throw("Ljava/lang/ArrayIndexOutOfBoundsException;", &format!("start {} len {}", start, len));
    }
}

unsafe extern "sysv64" fn register_natives(_e: usize, cls: usize, methods: *const usize, n: i32) -> i32 {
    let r = rt();
    let c = class_idx(cls);
    for i in 0..n.max(0) as usize {
        let name = cstring(*methods.add(i * 3) as *const u8);
        let sig = cstring(*methods.add(i * 3 + 1) as *const u8);
        let f = *methods.add(i * 3 + 2);
        let key = format!("{}{}", name, sig);
        match r.classes[c].methods.get(&key) {
            Some(m) if m.access & 0x100 != 0 => {
                r.jni.registered.insert((c, key), f);
            }
            _ => {
                throw("Ljava/lang/NoSuchMethodError;", &key);
                return -1;
            }
        }
    }
    0
}

unsafe extern "sysv64" fn unregister_natives(_e: usize, cls: usize) -> i32 {
    let c = class_idx(cls);
    rt().jni.registered.retain(|k, _| k.0 != c);
    0
}

unsafe extern "sysv64" fn monitor_enter_fn(_e: usize, obj: usize) -> i32 {
    let r = rt();
    if !r.th.mt || obj == 0 {
        return 0;
    }
    match r.monitor_enter(obj as Ref) {
        Ok(()) => 0,
        Err(e) => {
            set_pending(e);
            -1
        }
    }
}

unsafe extern "sysv64" fn monitor_exit_fn(_e: usize, obj: usize) -> i32 {
    let r = rt();
    if r.th.mt && obj != 0 {
        r.monitor_exit(obj as Ref);
    }
    0
}

unsafe extern "sysv64" fn get_java_vm(_e: usize, out: *mut usize) -> i32 {
    *out = vm_ptr();
    0
}

unsafe extern "sysv64" fn new_direct_buffer(_e: usize, _addr: usize, _cap: i64) -> usize {
    throw("Ljava/lang/UnsupportedOperationException;", "NewDirectByteBuffer");
    0
}

unsafe extern "sysv64" fn get_direct_address(_e: usize, _buf: usize) -> usize {
    0
}

unsafe extern "sysv64" fn get_direct_capacity(_e: usize, _buf: usize) -> i64 {
    -1
}

unsafe extern "sysv64" fn get_ref_type(_e: usize, obj: usize) -> i32 {
    (obj != 0) as i32
}

unsafe extern "sysv64" fn get_module(_e: usize, _cls: usize) -> usize {
    0
}

unsafe extern "sysv64" fn is_virtual_thread(_e: usize, _obj: usize) -> u8 {
    0
}
