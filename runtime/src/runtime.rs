use crate::dex::{Code, Dex, StaticVal, NO_INDEX};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub type Ref = u32;
pub type R<T> = Result<T, Ref>;

pub enum Kind {
    Plain,
    Str(Vec<u16>),
    Array(Vec<u64>),
    Class(usize),
}

pub struct Obj {
    pub class: usize,
    pub fields: Vec<u64>,
    pub kind: Kind,
}

pub struct Method {
    pub class: usize,
    pub dex: usize,
    pub name: String,
    pub key: String,
    pub access: u32,
    pub code: Option<Rc<Code>>,
}

pub struct Class {
    pub name: String,
    pub sup: Option<usize>,
    pub ifaces: Vec<usize>,
    pub access: u32,
    pub statics: Vec<u64>,
    pub static_idx: HashMap<String, usize>,
    pub inst_idx: HashMap<String, usize>,
    pub inst_total: usize,
    pub methods: HashMap<String, Rc<Method>>,
    pub init: u8,
    pub hook: u8,
}

struct MRef {
    class: String,
    key: String,
}

struct Frame {
    regs: Vec<u32>,
    pc: usize,
    ret: u64,
    exc: Ref,
}

pub struct Runtime {
    pub dexes: Vec<Rc<Dex>>,
    pub classes: Vec<Class>,
    by_name: HashMap<String, usize>,
    pub(crate) defs: HashMap<String, (usize, usize)>,
    pub heap: Vec<Obj>,
    pub(crate) interned: HashMap<String, Ref>,
    pub(crate) strl: Option<(usize, usize, Option<usize>, Option<usize>)>,

    class_objs: HashMap<usize, Ref>,
    mcache: HashMap<(usize, u32), Rc<MRef>>,
    sfcache: HashMap<(usize, u32), (usize, usize)>,
    ifcache: HashMap<(usize, u32), usize>,
    pub unresolved: HashSet<String>,
    depth: usize,
    trace: bool,
    ops: bool,
    pub last_text: String,
    pub log_text: bool,
    free: Vec<Ref>,
    allocs: usize,
    roots: Vec<Ref>,
    pub gc_runs: usize,
    pub arsc: Option<crate::arsc::Arsc>,
    pub apk: Option<crate::zip::Zip>,
    pub(crate) xml_docs: HashMap<u32, Rc<Vec<crate::axml::XmlEv>>>,
    pub(crate) xml_cur: Vec<Option<(Rc<Vec<crate::axml::XmlEv>>, usize)>>,
    pub text_log: Vec<String>,
    pub dump_buf: String,
    pub(crate) re_pats: Vec<Rc<crate::rx::Regex>>,
    pub(crate) re_ms: Vec<crate::rx::Matcher>,
    pub(crate) main_thread: u32,
}

pub const HOOK_STRING: u8 = 1;
pub const HOOK_FACTORY: u8 = 2;
pub const HOOK_OBJECT: u8 = 3;
pub const HOOK_CLASS: u8 = 4;
pub const HOOK_SYSTEM: u8 = 5;
pub const HOOK_MATH: u8 = 6;
pub const HOOK_BITS: u8 = 7;
pub const HOOK_UTF16: u8 = 8;
pub const HOOK_CHAR: u8 = 9;
pub const HOOK_THREAD: u8 = 10;
pub const HOOK_REGISTRY: u8 = 11;
pub const HOOK_PATTERN: u8 = 12;
pub const HOOK_MATCHER: u8 = 13;
pub const HOOK_REFERENCE: u8 = 14;
pub const HOOK_VMRUNTIME: u8 = 15;

pub(crate) fn hook_for(name: &str) -> u8 {
    match name {
        "Ljava/lang/String;" => HOOK_STRING,
        "Ljava/lang/StringFactory;" => HOOK_FACTORY,
        "Ljava/lang/Object;" => HOOK_OBJECT,
        "Ljava/lang/Class;" => HOOK_CLASS,
        "Ljava/lang/System;" => HOOK_SYSTEM,
        "Ljava/lang/Math;" | "Ljava/lang/StrictMath;" => HOOK_MATH,
        "Ljava/lang/Float;" | "Ljava/lang/Double;" => HOOK_BITS,
        "Ljava/lang/StringUTF16;" => HOOK_UTF16,
        "Ljava/lang/Character;" => HOOK_CHAR,
        "Ljava/lang/Thread;" => HOOK_THREAD,
        "Llibcore/util/NativeAllocationRegistry;" => HOOK_REGISTRY,
        "Lcom/android/icu/util/regex/PatternNative;" => HOOK_PATTERN,
        "Lcom/android/icu/util/regex/MatcherNative;" => HOOK_MATCHER,
        "Ljava/lang/ref/Reference;" => HOOK_REFERENCE,
        "Ldalvik/system/VMRuntime;" => HOOK_VMRUNTIME,
        _ => 0,
    }
}

fn builtin_super(name: &str) -> &'static str {
    match name {
        "Ljava/lang/StackOverflowError;" => "Ljava/lang/Error;",
        _ => "Ljava/lang/Object;",
    }
}

fn rw(f: &Frame, i: usize) -> u64 {
    (f.regs[i] as u64) | ((f.regs[i + 1] as u64) << 32)
}

fn ww(f: &mut Frame, i: usize, v: u64) {
    f.regs[i] = v as u32;
    f.regs[i + 1] = (v >> 32) as u32;
}

impl Runtime {
    pub fn new() -> Runtime {
        let mut rt = Runtime {
            dexes: vec![],
            classes: vec![],
            by_name: HashMap::new(),
            defs: HashMap::new(),
            heap: vec![],
            interned: HashMap::new(),
            strl: None,

            class_objs: HashMap::new(),
            mcache: HashMap::new(),
            sfcache: HashMap::new(),
            ifcache: HashMap::new(),
            unresolved: HashSet::new(),
            depth: 0,
            trace: std::env::var("WINART_TRACE").is_ok(),
            ops: std::env::var("WINART_OPS").is_ok(),
            last_text: String::new(),
            log_text: std::env::var("WINART_LOGTEXT").is_ok(),
            free: vec![],
            allocs: 0,
            roots: vec![],
            gc_runs: 0,
            arsc: None,
            apk: None,
            xml_docs: HashMap::new(),
            xml_cur: Vec::new(),
            text_log: Vec::new(),
            dump_buf: String::new(),
            re_pats: Vec::new(),
            re_ms: Vec::new(),
            main_thread: 0,
        };
        rt.heap.push(Obj { class: 0, fields: vec![], kind: Kind::Plain });

        rt
    }

    pub fn start(&mut self) {
        self.load("Ljava/lang/Object;");
    }

    pub fn add_dex(&mut self, dex: Dex) {
        let di = self.dexes.len();
        for (ci, c) in dex.classes.iter().enumerate() {
            let name = dex.type_desc(c.class).to_string();
            self.defs.entry(name).or_insert((di, ci));
        }
        self.dexes.push(Rc::new(dex));
    }

    pub fn load(&mut self, name: &str) -> usize {
        if let Some(&i) = self.by_name.get(name) {
            return i;
        }
        if let Some(&(d, c)) = self.defs.get(name) {
            return self.load_dex_class(name, d, c);
        }
        let sup = if name == "Ljava/lang/Object;" {
            None
        } else if name.starts_with('[') {
            Some(self.load("Ljava/lang/Object;"))
        } else {
            Some(self.load(builtin_super(name)))
        };
        let (idx_map, total) = match sup {
            Some(s) => (self.classes[s].inst_idx.clone(), self.classes[s].inst_total),
            None => (HashMap::new(), 0),
        };
        let idx = self.classes.len();
        self.classes.push(Class {
            name: name.to_string(),
            sup,
            ifaces: vec![],
            access: 1,
            statics: vec![],
            static_idx: HashMap::new(),
            inst_idx: idx_map,
            inst_total: total,
            methods: HashMap::new(),
            init: 2,
            hook: 0,
        });
        self.by_name.insert(name.to_string(), idx);
        idx
    }

    fn load_dex_class(&mut self, name: &str, d: usize, c: usize) -> usize {
        let dex = self.dexes[d].clone();
        let def = &dex.classes[c];
        let sup = if def.super_ == NO_INDEX { None } else { Some(self.load(dex.type_desc(def.super_))) };
        let ifaces: Vec<usize> = def.ifaces.iter().map(|t| self.load(dex.type_desc(*t as u32))).collect();
        let cd = dex.class_data(def.class_data_off);
        let (mut idx_map, base) = match sup {
            Some(s) => (self.classes[s].inst_idx.clone(), self.classes[s].inst_total),
            None => (HashMap::new(), 0),
        };
        for (i, f) in cd.instances.iter().enumerate() {
            idx_map.insert(dex.field_name(f.idx).to_string(), base + i);
        }
        let mut static_idx = HashMap::new();
        for (i, f) in cd.statics.iter().enumerate() {
            static_idx.insert(dex.field_name(f.idx).to_string(), i);
        }
        let idx = self.classes.len();
        let mut methods = HashMap::new();
        for em in cd.directs.iter().chain(cd.virtuals.iter()) {
            let mid = &dex.methods[em.idx as usize];
            let mname = dex.method_name(em.idx).to_string();
            let key = format!("{}{}", mname, dex.sig(mid.proto));
            let code = if em.code_off != 0 { Some(Rc::new(dex.code(em.code_off))) } else { None };
            methods.insert(key.clone(), Rc::new(Method { class: idx, dex: d, name: mname, key, access: em.access, code }));
        }
        self.classes.push(Class {
            name: name.to_string(),
            sup,
            ifaces,
            access: def.access,
            statics: vec![0; cd.statics.len()],
            static_idx,
            inst_idx: idx_map,
            inst_total: base + cd.instances.len(),
            methods,
            init: 0,
            hook: hook_for(name),
        });
        self.by_name.insert(name.to_string(), idx);
        let vals = dex.static_values(def.static_values_off);
        for (i, v) in vals.iter().enumerate() {
            if i >= cd.statics.len() {
                break;
            }
            let raw = match v {
                StaticVal::Int(x) => *x,
                StaticVal::Str(s) => self.str_ref(&dex.strings[*s as usize]) as u64,
                StaticVal::Null => 0,
            };
            self.classes[idx].statics[i] = raw;
        }
        idx
    }

    pub(crate) fn ensure_init(&mut self, c: usize) -> R<()> {
        if self.classes[c].init != 0 {
            return Ok(());
        }
        self.classes[c].init = 1;
        if let Some(s) = self.classes[c].sup {
            self.ensure_init(s)?;
        }
        if self.classes[c].hook == HOOK_SYSTEM {
            self.system_init(c);
            self.classes[c].init = 2;
            return Ok(());
        }
        if let Some(m) = self.classes[c].methods.get("<clinit>()V").cloned() {
            self.invoke(&m, vec![])?;
        }
        self.classes[c].init = 2;
        Ok(())
    }

    pub(crate) fn alloc(&mut self, o: Obj) -> Ref {
        self.allocs += 1;
        if let Some(i) = self.free.pop() {
            self.heap[i as usize] = o;
            return i;
        }
        self.heap.push(o);
        (self.heap.len() - 1) as Ref
    }

    pub fn add_root(&mut self, r: Ref) {
        self.roots.push(r);
    }

    pub fn live_objects(&self) -> usize {
        self.heap.len() - self.free.len()
    }

    pub fn maybe_gc(&mut self) {
        if self.depth == 0 && self.allocs > 20000 && std::env::var("WINART_NOGC").is_err() {
            self.gc();
        }
    }

    pub fn gc(&mut self) -> usize {
        if self.depth != 0 {
            return 0;
        }
        let n = self.heap.len();
        let mut mark = vec![false; n];
        let mut work: Vec<usize> = Vec::new();
        let push = |v: u64, mark: &mut Vec<bool>, work: &mut Vec<usize>| {
            if v > 0 && (v as usize) < n && !mark[v as usize] {
                mark[v as usize] = true;
                work.push(v as usize);
            }
        };
        for &r in &self.roots {
            push(r as u64, &mut mark, &mut work);
        }
        for &r in self.interned.values() {
            push(r as u64, &mut mark, &mut work);
        }
        for &r in self.class_objs.values() {
            push(r as u64, &mut mark, &mut work);
        }
        for c in &self.classes {
            for &v in &c.statics {
                push(v, &mut mark, &mut work);
            }
        }
        while let Some(i) = work.pop() {
            let o = &self.heap[i];
            if !matches!(o.kind, Kind::Str(_)) {
                for &v in &o.fields {
                    push(v, &mut mark, &mut work);
                }
            }
            if let Kind::Array(a) = &o.kind {
                let prim = matches!(self.classes[o.class].name.as_bytes().get(1), Some(b'Z' | b'B' | b'C' | b'S' | b'I' | b'J' | b'F' | b'D'));
                for &v in a.iter().filter(|_| !prim) {
                    push(v, &mut mark, &mut work);
                }
            }
        }
        let mut freed = 0;
        self.free.clear();
        for i in 1..n {
            if !mark[i] {
                self.heap[i] = Obj { class: 0, fields: Vec::new(), kind: Kind::Plain };
                self.free.push(i as Ref);
                freed += 1;
            }
        }
        self.allocs = 0;
        self.gc_runs += 1;
        freed
    }

    pub(crate) fn str_layout(&mut self) -> (usize, usize, Option<usize>, Option<usize>) {
        if let Some(l) = self.strl {
            return l;
        }
        let c = self.load("Ljava/lang/String;");
        let cl = &self.classes[c];
        let l = (c, cl.inst_total, cl.inst_idx.get("count").copied(), cl.inst_idx.get("hash").copied());
        self.strl = Some(l);
        l
    }

    pub fn new_string(&mut self, u: Vec<u16>) -> Ref {
        let (c, total, count_slot, _) = self.str_layout();
        let mut fields = vec![0u64; total];
        if let Some(s) = count_slot {
            let compressed = u.iter().all(|&x| x != 0 && x < 0x80);
            fields[s] = ((u.len() as u64) << 1) | (!compressed) as u64;
        }
        self.alloc(Obj { class: c, fields, kind: Kind::Str(u) })
    }

    pub(crate) fn new_array(&mut self, desc: &str, vals: Vec<u64>) -> Ref {
        let c = self.load(desc);
        self.alloc(Obj { class: c, fields: vec![], kind: Kind::Array(vals) })
    }

    pub(crate) fn throw_msg(&mut self, name: &str, msg: &str) -> Ref {
        let e = self.throw_new(name);
        let m = self.new_string(msg.encode_utf16().collect());
        let c = self.heap[e as usize].class;
        if let Some(&slot) = self.classes[c].inst_idx.get("detailMessage") {
            if slot < self.heap[e as usize].fields.len() {
                self.heap[e as usize].fields[slot] = m as u64;
            }
        }
        e
    }

    pub fn str_ref(&mut self, s: &str) -> Ref {
        if let Some(&r) = self.interned.get(s) {
            return r;
        }
        let r = self.new_string(s.encode_utf16().collect());
        self.interned.insert(s.to_string(), r);
        r
    }

    pub fn string_value(&self, r: Ref) -> String {
        match &self.heap[r as usize].kind {
            Kind::Str(u) => String::from_utf16_lossy(u),
            _ => String::new(),
        }
    }

    pub fn class_name_of(&self, r: Ref) -> String {
        self.classes[self.heap[r as usize].class].name.clone()
    }

    pub(crate) fn throw_new(&mut self, name: &str) -> Ref {
        let c = self.load(name);
        let n = self.classes[c].inst_total;
        self.alloc(Obj { class: c, fields: vec![0; n], kind: Kind::Plain })
    }

    pub(crate) fn npe(&mut self) -> Ref {
        self.throw_new("Ljava/lang/NullPointerException;")
    }

    pub(crate) fn is_sub(&self, c: usize, t: usize) -> bool {
        if c == t {
            return true;
        }
        let cl = &self.classes[c];
        if let Some(s) = cl.sup {
            if self.is_sub(s, t) {
                return true;
            }
        }
        cl.ifaces.iter().any(|&i| self.is_sub(i, t))
    }

    pub(crate) fn find_method(&self, c: usize, key: &str) -> Option<Rc<Method>> {
        let mut cur = Some(c);
        while let Some(ci) = cur {
            if let Some(m) = self.classes[ci].methods.get(key) {
                return Some(m.clone());
            }
            cur = self.classes[ci].sup;
        }
        let mut cur = Some(c);
        while let Some(ci) = cur {
            for &i in &self.classes[ci].ifaces {
                if let Some(m) = self.find_method(i, key) {
                    return Some(m);
                }
            }
            cur = self.classes[ci].sup;
        }
        None
    }

    fn find_static(&self, c: usize, name: &str) -> Option<(usize, usize)> {
        let mut cur = Some(c);
        while let Some(ci) = cur {
            if let Some(&s) = self.classes[ci].static_idx.get(name) {
                return Some((ci, s));
            }
            for &i in &self.classes[ci].ifaces {
                if let Some(r) = self.find_static(i, name) {
                    return Some(r);
                }
            }
            cur = self.classes[ci].sup;
        }
        None
    }

    fn mref(&mut self, di: usize, idx: u32) -> Rc<MRef> {
        if let Some(m) = self.mcache.get(&(di, idx)) {
            return m.clone();
        }
        let dex = self.dexes[di].clone();
        let mid = &dex.methods[idx as usize];
        let r = Rc::new(MRef {
            class: dex.type_desc(mid.class as u32).to_string(),
            key: format!("{}{}", dex.method_name(idx), dex.sig(mid.proto)),
        });
        self.mcache.insert((di, idx), r.clone());
        r
    }

    fn static_slot(&mut self, di: usize, idx: u32) -> R<(usize, usize)> {
        if let Some(&r) = self.sfcache.get(&(di, idx)) {
            return Ok(r);
        }
        let dex = self.dexes[di].clone();
        let f = &dex.fields[idx as usize];
        let c = self.load(dex.type_desc(f.class as u32));
        match self.find_static(c, dex.field_name(idx)) {
            Some(r) => {
                self.ensure_init(r.0)?;
                self.sfcache.insert((di, idx), r);
                Ok(r)
            }
            None => {
                eprintln!("UNRESOLVED static field {}.{}", dex.type_desc(f.class as u32), dex.field_name(idx));
                Err(self.throw_new("Ljava/lang/NoSuchFieldError;"))
            }
        }
    }

    fn inst_slot(&mut self, di: usize, idx: u32) -> R<usize> {
        if let Some(&r) = self.ifcache.get(&(di, idx)) {
            return Ok(r);
        }
        let dex = self.dexes[di].clone();
        let f = &dex.fields[idx as usize];
        let c = self.load(dex.type_desc(f.class as u32));
        match self.classes[c].inst_idx.get(dex.field_name(idx)).copied() {
            Some(s) => {
                self.ifcache.insert((di, idx), s);
                Ok(s)
            }
            None => {
                eprintln!("UNRESOLVED instance field {}.{}", dex.type_desc(f.class as u32), dex.field_name(idx));
                Err(self.throw_new("Ljava/lang/NoSuchFieldError;"))
            }
        }
    }

    pub(crate) fn class_obj(&mut self, c: usize) -> Ref {
        if let Some(&r) = self.class_objs.get(&c) {
            return r;
        }
        let cc = self.load("Ljava/lang/Class;");
        let r = self.alloc(Obj { class: cc, fields: vec![], kind: Kind::Class(c) });
        self.class_objs.insert(c, r);
        r
    }

    fn native(&mut self, key: &str, args: &[u32]) -> Option<R<u64>> {
        if let Some(r) = self.hooked_native(key, args) {
            return Some(r);
        }
        match key {
            "Ljava/lang/Object;-><init>()V" => Some(Ok(0)),
            "Landroid/winrt/Native;->fillRect(IIIII)V" => {
                crate::ui::fill_rect(args[0] as i32, args[1] as i32, args[2] as i32, args[3] as i32, args[4] as i32);
                Some(Ok(0))
            }
            "Landroid/winrt/Native;->strokeRect(IIIIII)V" => {
                crate::ui::stroke_rect(args[0] as i32, args[1] as i32, args[2] as i32, args[3] as i32, args[4] as i32, args[5] as i32);
                Some(Ok(0))
            }
            "Landroid/winrt/Native;->drawText(Ljava/lang/String;IIII)V" => {
                let s = self.string_value(args[0]);
                crate::ui::draw_text(&s, args[1] as i32, args[2] as i32, args[3] as i32, args[4] as i32);
                Some(Ok(0))
            }
            "Landroid/winrt/Native;->measureText(Ljava/lang/String;I)I" => {
                let s = self.string_value(args[0]);
                Some(Ok(crate::ui::measure_text(&s, args[1] as i32) as u32 as u64))
            }
            "Landroid/winrt/Native;->invalidate()V" => {
                crate::ui::invalidate();
                Some(Ok(0))
            }
            "Landroid/winrt/Native;->setTitle(Ljava/lang/String;)V" => {
                let s = self.string_value(args[0]);
                crate::ui::set_title(&s);
                Some(Ok(0))
            }
            "Landroid/winrt/Native;->exit()V" => {
                crate::ui::exit();
                Some(Ok(0))
            }
            "Ldalvik/system/VMStack;->getStackClass2()Ljava/lang/Class;" => {
                let o = self.load("Ljava/lang/Object;");
                Some(Ok(self.class_obj(o) as u64))
            }
            "Ljava/lang/Throwable;->nativeFillInStackTrace()Ljava/lang/Object;" => Some(Ok(0)),
            "Ljava/lang/Throwable;->nativeGetStackTrace(Ljava/lang/Object;)[Ljava/lang/StackTraceElement;" => Some(Ok(0)),
            "Ljava/lang/Class;->getPrimitiveClass(Ljava/lang/String;)Ljava/lang/Class;" => {
                let n = self.string_value(args[0]);
                let d = match n.as_str() {
                    "int" => "I",
                    "long" => "J",
                    "short" => "S",
                    "byte" => "B",
                    "char" => "C",
                    "boolean" => "Z",
                    "float" => "F",
                    "double" => "D",
                    _ => "V",
                };
                let c = self.load(d);
                Some(Ok(self.class_obj(c) as u64))
            }

            _ => self.res_native(key, args),
        }
    }

    fn call_native(&mut self, class: &str, key: &str, args: &[u32]) -> R<u64> {
        let full = format!("{}->{}", class, key);
        if let Some(r) = self.native(&full, args) {
            return r;
        }
        if self.unresolved.insert(full.clone()) {
            eprintln!("UNRESOLVED method {}", full);
        }
        Err(self.throw_new("Ljava/lang/NoSuchMethodError;"))
    }

    pub fn invoke(&mut self, m: &Rc<Method>, args: Vec<u32>) -> R<u64> {
        if self.depth > 600 {
            return Err(self.throw_new("Ljava/lang/StackOverflowError;"));
        }
        self.depth += 1;
        if m.code.is_some() {
            let h = self.classes[m.class].hook;
            if h != 0 {
                if let Some(r) = self.hook_native(h, &m.key, &args) {
                    self.depth -= 1;
                    return r;
                }
            }
        }
        if m.key == "setText(Ljava/lang/CharSequence;)V" && args.len() > 1 && self.classes[m.class].name == "Landroid/widget/TextView;" {
            let t = if args[1] == 0 { String::new() } else { self.string_value(args[1]) };
            if self.log_text {
                eprintln!("[setText] #{} {:?}", args[0], t);
            }
            if self.text_log.len() > 20000 {
                self.text_log.drain(..10000);
            }
            self.text_log.push(t.clone());
            self.last_text = t;
        }
        if self.trace {

            eprintln!("{}{}->{}", " ".repeat(self.depth.min(40)), self.classes[m.class].name, m.key);
        }
        let r = match &m.code {
            Some(_) => self.exec(m, args),
            None => {
                let cn = self.classes[m.class].name.clone();
                self.call_native(&cn, &m.key, &args)
            }
        };
        self.depth -= 1;
        r
    }

    pub(crate) fn arr_vec(&self, r: u32) -> Vec<u64> {
        match &self.heap[r as usize].kind {
            Kind::Array(v) => v.clone(),
            _ => vec![],
        }
    }

    fn invoke_site(&mut self, m: &Rc<Method>, f: &mut Frame, kind: u32, midx: u32, args: Vec<u32>) -> R<()> {
        if kind == 0x70 {
            if let Some(k) = self.string_init_key(m.dex, midx) {
                let old = args[0];
                let nr = self.string_init(&k, args)?;
                for x in f.regs.iter_mut() {
                    if *x == old {
                        *x = nr;
                    }
                }
                f.ret = 0;
                return Ok(());
            }
        }
        f.ret = self.invoke_ref(m, m.dex, kind, midx, args)?;
        Ok(())
    }

    fn string_init_key(&mut self, di: usize, midx: u32) -> Option<String> {
        let mr = self.mref(di, midx);
        if mr.class == "Ljava/lang/String;" && mr.key.starts_with("<init>") {
            Some(mr.key.clone())
        } else {
            None
        }
    }

    fn string_init(&mut self, key: &str, args: Vec<u32>) -> R<u32> {
        let sf = self.load("Ljava/lang/StringFactory;");
        self.ensure_init(sf)?;
        let params = key["<init>".len()..].to_string();
        let params = params[..params.find(')').unwrap_or(0) + 1].to_string();
        let found = self.classes[sf]
            .methods
            .values()
            .find(|m| m.key.ends_with(")Ljava/lang/String;") && m.key[m.name.len()..].starts_with(&params) && m.name.starts_with("new") && !m.name.contains("Utf"))
            .cloned();
        match found {
            Some(m) => Ok(self.invoke(&m, args[1..].to_vec())? as u32),
            None => {
                if self.unresolved.insert(format!("StringFactory for {}", key)) {
                    eprintln!("UNRESOLVED StringFactory for String.{}", key);
                }
                Err(self.throw_new("Ljava/lang/NoSuchMethodError;"))
            }
        }
    }

    fn invoke_ref(&mut self, cur: &Rc<Method>, di: usize, kind: u32, midx: u32, args: Vec<u32>) -> R<u64> {
        let mr = self.mref(di, midx);
        match kind {
            0x71 => {
                let c = self.load(&mr.class);
                self.ensure_init(c)?;
                match self.find_method(c, &mr.key) {
                    Some(m) => self.invoke(&m, args),
                    None => self.call_native(&mr.class, &mr.key, &args),
                }
            }
            0x70 => {
                let c = self.load(&mr.class);
                match self.find_method(c, &mr.key) {
                    Some(m) => self.invoke(&m, args),
                    None => self.call_native(&mr.class, &mr.key, &args),
                }
            }
            0x6f => {
                let start = self.classes[cur.class].sup.unwrap_or_else(|| self.load(&mr.class));
                match self.find_method(start, &mr.key) {
                    Some(m) => self.invoke(&m, args),
                    None => self.call_native(&mr.class, &mr.key, &args),
                }
            }
            _ => {
                if args[0] == 0 {
                    return Err(self.npe());
                }
                let rc = self.heap[args[0] as usize].class;
                if let Some(m) = self.find_method(rc, &mr.key) {
                    return self.invoke(&m, args);
                }
                let mut cur = Some(rc);
                while let Some(ci) = cur {
                    let full = format!("{}->{}", self.classes[ci].name, mr.key);
                    if let Some(r) = self.native(&full, &args) {
                        return r;
                    }
                    cur = self.classes[ci].sup;
                }
                self.call_native(&mr.class, &mr.key, &args)
            }
        }
    }

    pub fn new_instance(&mut self, class: &str) -> R<Ref> {
        let c = self.load(class);
        self.ensure_init(c)?;
        let n = self.classes[c].inst_total;
        let o = self.alloc(Obj { class: c, fields: vec![0; n], kind: Kind::Plain });
        if let Some(m) = self.find_method(c, "<init>()V") {
            self.invoke(&m, vec![o])?;
        }
        Ok(o)
    }

    pub fn call_static(&mut self, class: &str, key: &str, args: Vec<u32>) -> R<u64> {
        let c = self.load(class);
        self.ensure_init(c)?;
        match self.find_method(c, key) {
            Some(m) => self.invoke(&m, args),
            None => self.call_native(class, key, &args),
        }
    }

    pub fn call_virtual(&mut self, obj: Ref, key: &str, mut args: Vec<u32>) -> R<u64> {
        let rc = self.heap[obj as usize].class;
        args.insert(0, obj);
        match self.find_method(rc, key) {
            Some(m) => self.invoke(&m, args),
            None => {
                let cn = self.classes[rc].name.clone();
                self.call_native(&cn, key, &args)
            }
        }
    }

    fn exec(&mut self, m: &Rc<Method>, args: Vec<u32>) -> R<u64> {
        let code = m.code.clone().unwrap();
        let dex = self.dexes[m.dex].clone();
        let n = (code.regs as usize).max(code.ins as usize);
        let mut f = Frame { regs: vec![0; n], pc: 0, ret: 0, exc: 0 };
        let base = n - code.ins as usize;
        for (k, v) in args.iter().enumerate() {
            if base + k < n {
                f.regs[base + k] = *v;
            }
        }
        loop {
            match self.step(m, &dex, &code, &mut f) {
                Ok(Some(v)) => return Ok(v),
                Ok(None) => {}
                Err(e) => match self.find_handler(&dex, &code, f.pc, e) {
                    Some(a) => {
                        f.exc = e;
                        f.pc = a;
                    }
                    None => return Err(e),
                },
            }
        }
    }

    fn find_handler(&mut self, dex: &Rc<Dex>, code: &Code, pc: usize, exc: Ref) -> Option<usize> {
        let ec = self.heap[exc as usize].class;
        for t in &code.tries {
            if (pc as u32) >= t.start && (pc as u32) < t.start + t.count {
                let h = &code.handlers[&t.handler];
                for &(ty, addr) in &h.catches {
                    let tc = self.load(dex.type_desc(ty));
                    if self.is_sub(ec, tc) {
                        return Some(addr as usize);
                    }
                }
                if let Some(a) = h.catch_all {
                    return Some(a as usize);
                }
            }
        }
        None
    }

    fn int_op(&mut self, k: u32, a: i32, b: i32) -> R<i32> {
        Ok(match k {
            0 => a.wrapping_add(b),
            1 => a.wrapping_sub(b),
            2 => a.wrapping_mul(b),
            3 => {
                if b == 0 {
                    return Err(self.throw_new("Ljava/lang/ArithmeticException;"));
                }
                a.wrapping_div(b)
            }
            4 => {
                if b == 0 {
                    return Err(self.throw_new("Ljava/lang/ArithmeticException;"));
                }
                a.wrapping_rem(b)
            }
            5 => a & b,
            6 => a | b,
            7 => a ^ b,
            8 => a.wrapping_shl(b as u32 & 31),
            9 => a.wrapping_shr(b as u32 & 31),
            _ => ((a as u32).wrapping_shr(b as u32 & 31)) as i32,
        })
    }

    fn long_op(&mut self, k: u32, a: i64, b: i64) -> R<i64> {
        Ok(match k {
            0 => a.wrapping_add(b),
            1 => a.wrapping_sub(b),
            2 => a.wrapping_mul(b),
            3 => {
                if b == 0 {
                    return Err(self.throw_new("Ljava/lang/ArithmeticException;"));
                }
                a.wrapping_div(b)
            }
            4 => {
                if b == 0 {
                    return Err(self.throw_new("Ljava/lang/ArithmeticException;"));
                }
                a.wrapping_rem(b)
            }
            5 => a & b,
            6 => a | b,
            7 => a ^ b,
            8 => a.wrapping_shl(b as u32 & 63),
            9 => a.wrapping_shr(b as u32 & 63),
            _ => ((a as u64).wrapping_shr(b as u32 & 63)) as i64,
        })
    }

    fn binop(&mut self, op: u32, x: u64, y: u64) -> R<u64> {
        Ok(match op {
            0x90..=0x9a => self.int_op(op - 0x90, x as i32, y as i32)? as u32 as u64,
            0x9b..=0xa5 => self.long_op(op - 0x9b, x as i64, y as i64)? as u64,
            0xa6..=0xaa => {
                let (a, b) = (f32::from_bits(x as u32), f32::from_bits(y as u32));
                let r = match op - 0xa6 {
                    0 => a + b,
                    1 => a - b,
                    2 => a * b,
                    3 => a / b,
                    _ => a % b,
                };
                r.to_bits() as u64
            }
            _ => {
                let (a, b) = (f64::from_bits(x), f64::from_bits(y));
                let r = match op - 0xab {
                    0 => a + b,
                    1 => a - b,
                    2 => a * b,
                    3 => a / b,
                    _ => a % b,
                };
                r.to_bits()
            }
        })
    }

    fn array_get(&mut self, a: Ref, i: i32) -> R<u64> {
        if a == 0 {
            return Err(self.npe());
        }
        let r = match &self.heap[a as usize].kind {
            Kind::Array(v) => {
                if i >= 0 && (i as usize) < v.len() {
                    Some(v[i as usize])
                } else {
                    None
                }
            }
            _ => None,
        };
        match r {
            Some(v) => Ok(v),
            None => Err(self.throw_new("Ljava/lang/ArrayIndexOutOfBoundsException;")),
        }
    }

    fn array_set(&mut self, a: Ref, i: i32, val: u64) -> R<()> {
        if a == 0 {
            return Err(self.npe());
        }
        let ok = match &mut self.heap[a as usize].kind {
            Kind::Array(v) => {
                if i >= 0 && (i as usize) < v.len() {
                    v[i as usize] = val;
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        if ok {
            Ok(())
        } else {
            Err(self.throw_new("Ljava/lang/ArrayIndexOutOfBoundsException;"))
        }
    }

    fn step(&mut self, m: &Rc<Method>, dex: &Rc<Dex>, code: &Rc<Code>, f: &mut Frame) -> R<Option<u64>> {
        let pc = f.pc;
        let ins = &code.insns;
        let i = ins[pc] as u32;
        let op = i & 0xff;
        if self.ops {
            eprintln!("  @{:04x} op {:02x} {}", pc, op, m.key);
        }
        let a = ((i >> 8) & 0xf) as usize;
        let b = ((i >> 12) & 0xf) as usize;
        let aa = (i >> 8) as usize;
        let g = |k: usize| ins.get(pc + k).copied().unwrap_or(0) as u32;
        let u1 = g(1);
        let u2 = g(2);
        let u3 = g(3);
        let u4 = g(4);
        let mut len = 1usize;
        macro_rules! r {
            ($x:expr) => {
                f.regs[$x as usize]
            };
        }
        match op {
            0x00 => {}
            0x01 | 0x07 => r!(a) = r!(b),
            0x02 | 0x08 => {
                r!(aa) = r!(u1);
                len = 2;
            }
            0x03 | 0x09 => {
                r!(u1) = r!(u2);
                len = 3;
            }
            0x04 => {
                let v = rw(f, b);
                ww(f, a, v);
            }
            0x05 => {
                let v = rw(f, u1 as usize);
                ww(f, aa, v);
                len = 2;
            }
            0x06 => {
                let v = rw(f, u2 as usize);
                ww(f, u1 as usize, v);
                len = 3;
            }
            0x0a | 0x0c => r!(aa) = f.ret as u32,
            0x0b => {
                let v = f.ret;
                ww(f, aa, v);
            }
            0x0d => r!(aa) = f.exc,
            0x0e => return Ok(Some(0)),
            0x0f | 0x11 => return Ok(Some(r!(aa) as u64)),
            0x10 => return Ok(Some(rw(f, aa))),
            0x12 => {
                let v = ((i >> 12) & 0xf) as i32;
                r!(a) = ((v << 28) >> 28) as u32;
            }
            0x13 => {
                r!(aa) = (u1 as u16 as i16 as i32) as u32;
                len = 2;
            }
            0x14 => {
                r!(aa) = u1 | (u2 << 16);
                len = 3;
            }
            0x15 => {
                r!(aa) = u1 << 16;
                len = 2;
            }
            0x16 => {
                ww(f, aa, (u1 as u16 as i16 as i64) as u64);
                len = 2;
            }
            0x17 => {
                ww(f, aa, ((u1 | (u2 << 16)) as i32 as i64) as u64);
                len = 3;
            }
            0x18 => {
                ww(f, aa, (u1 as u64) | ((u2 as u64) << 16) | ((u3 as u64) << 32) | ((u4 as u64) << 48));
                len = 5;
            }
            0x19 => {
                ww(f, aa, (u1 as u64) << 48);
                len = 2;
            }
            0x1a | 0x1b => {
                let idx = if op == 0x1a { u1 } else { u1 | (u2 << 16) };
                len = if op == 0x1a { 2 } else { 3 };
                let s = dex.strings[idx as usize].clone();
                r!(aa) = self.str_ref(&s);
            }
            0x1c => {
                let c = self.load(dex.type_desc(u1));
                r!(aa) = self.class_obj(c);
                len = 2;
            }
            0x1d | 0x1e => {}
            0x1f => {
                let o = r!(aa);
                if o != 0 {
                    let t = self.load(dex.type_desc(u1));
                    if !self.is_sub(self.heap[o as usize].class, t) {
                        return Err(self.throw_new("Ljava/lang/ClassCastException;"));
                    }
                }
                len = 2;
            }
            0x20 => {
                let o = r!(b);
                let t = self.load(dex.type_desc(u1));
                r!(a) = (o != 0 && self.is_sub(self.heap[o as usize].class, t)) as u32;
                len = 2;
            }
            0x21 => {
                let o = r!(b);
                if o == 0 {
                    return Err(self.npe());
                }
                r!(a) = match &self.heap[o as usize].kind {
                    Kind::Array(v) => v.len() as u32,
                    _ => 0,
                };
            }
            0x22 => {
                let c = self.load(dex.type_desc(u1));
                self.ensure_init(c)?;
                let n = self.classes[c].inst_total;
                r!(aa) = self.alloc(Obj { class: c, fields: vec![0; n], kind: Kind::Plain });
                len = 2;
            }
            0x23 => {
                let n = r!(b) as i32;
                if n < 0 {
                    return Err(self.throw_new("Ljava/lang/NegativeArraySizeException;"));
                }
                let c = self.load(dex.type_desc(u1));
                r!(a) = self.alloc(Obj { class: c, fields: vec![], kind: Kind::Array(vec![0; n as usize]) });
                len = 2;
            }
            0x24 | 0x25 => {
                let c = self.load(dex.type_desc(u1));
                let cnt = if op == 0x24 { b } else { aa };
                let regs: Vec<usize> = if op == 0x24 {
                    let all = [(u2 & 0xf) as usize, ((u2 >> 4) & 0xf) as usize, ((u2 >> 8) & 0xf) as usize, ((u2 >> 12) & 0xf) as usize, a];
                    all[..cnt].to_vec()
                } else {
                    (0..cnt).map(|k| u2 as usize + k).collect()
                };
                let wide = self.classes[c].name == "[J" || self.classes[c].name == "[D";
                let mut vals = Vec::new();
                let mut k = 0;
                while k < regs.len() {
                    if wide {
                        vals.push(rw(f, regs[k]));
                        k += 2;
                    } else {
                        vals.push(f.regs[regs[k]] as u64);
                        k += 1;
                    }
                }
                f.ret = self.alloc(Obj { class: c, fields: vec![], kind: Kind::Array(vals) }) as u64;
                len = 3;
            }
            0x26 => {
                let off = (u1 | (u2 << 16)) as i32;
                let p = (pc as i64 + off as i64) as usize;
                let width = ins[p + 1] as usize;
                let size = ins[p + 2] as usize | ((ins[p + 3] as usize) << 16);
                let mut vals = Vec::with_capacity(size);
                let mut bp = (p + 4) * 2;
                for _ in 0..size {
                    let mut v: u64 = 0;
                    for k in 0..width {
                        let byte = (ins[(bp + k) / 2] >> (8 * ((bp + k) % 2))) as u64 & 0xff;
                        v |= byte << (8 * k);
                    }
                    bp += width;
                    vals.push(v);
                }
                let arr = r!(aa);
                if arr == 0 {
                    return Err(self.npe());
                }
                if let Kind::Array(v) = &mut self.heap[arr as usize].kind {
                    for (k, x) in vals.into_iter().enumerate() {
                        if k < v.len() {
                            v[k] = x;
                        }
                    }
                }
                len = 3;
            }
            0x27 => {
                let o = r!(aa);
                if o == 0 {
                    return Err(self.npe());
                }
                return Err(o);
            }
            0x28 => {
                f.pc = (pc as i64 + (i >> 8) as u8 as i8 as i64) as usize;
                return Ok(None);
            }
            0x29 => {
                f.pc = (pc as i64 + u1 as u16 as i16 as i64) as usize;
                return Ok(None);
            }
            0x2a => {
                f.pc = (pc as i64 + (u1 | (u2 << 16)) as i32 as i64) as usize;
                return Ok(None);
            }
            0x2b | 0x2c => {
                let off = (u1 | (u2 << 16)) as i32;
                let p = (pc as i64 + off as i64) as usize;
                let val = r!(aa) as i32;
                let size = ins[p + 1] as usize;
                let mut target: i32 = 3;
                if op == 0x2b {
                    let first = (ins[p + 2] as u32 | ((ins[p + 3] as u32) << 16)) as i32;
                    let idx = val.wrapping_sub(first);
                    if idx >= 0 && (idx as usize) < size {
                        let q = p + 4 + idx as usize * 2;
                        target = (ins[q] as u32 | ((ins[q + 1] as u32) << 16)) as i32;
                    }
                } else {
                    for k in 0..size {
                        let kq = p + 2 + k * 2;
                        let key = (ins[kq] as u32 | ((ins[kq + 1] as u32) << 16)) as i32;
                        if key == val {
                            let tq = p + 2 + size * 2 + k * 2;
                            target = (ins[tq] as u32 | ((ins[tq + 1] as u32) << 16)) as i32;
                            break;
                        }
                    }
                }
                f.pc = (pc as i64 + target as i64) as usize;
                return Ok(None);
            }
            0x2d..=0x31 => {
                let (bb, cc) = ((u1 & 0xff) as usize, (u1 >> 8) as usize);
                let res: i32 = match op {
                    0x2d | 0x2e => {
                        let (x, y) = (f32::from_bits(r!(bb)), f32::from_bits(r!(cc)));
                        if x.is_nan() || y.is_nan() {
                            if op == 0x2e { 1 } else { -1 }
                        } else if x > y {
                            1
                        } else if x == y {
                            0
                        } else {
                            -1
                        }
                    }
                    0x2f | 0x30 => {
                        let (x, y) = (f64::from_bits(rw(f, bb)), f64::from_bits(rw(f, cc)));
                        if x.is_nan() || y.is_nan() {
                            if op == 0x30 { 1 } else { -1 }
                        } else if x > y {
                            1
                        } else if x == y {
                            0
                        } else {
                            -1
                        }
                    }
                    _ => {
                        let (x, y) = (rw(f, bb) as i64, rw(f, cc) as i64);
                        if x > y {
                            1
                        } else if x == y {
                            0
                        } else {
                            -1
                        }
                    }
                };
                r!(aa) = res as u32;
                len = 2;
            }
            0x32..=0x37 => {
                let (x, y) = (r!(a) as i32, r!(b) as i32);
                let t = match op {
                    0x32 => x == y,
                    0x33 => x != y,
                    0x34 => x < y,
                    0x35 => x >= y,
                    0x36 => x > y,
                    _ => x <= y,
                };
                if t {
                    f.pc = (pc as i64 + u1 as u16 as i16 as i64) as usize;
                    return Ok(None);
                }
                len = 2;
            }
            0x38..=0x3d => {
                let x = r!(aa) as i32;
                let t = match op {
                    0x38 => x == 0,
                    0x39 => x != 0,
                    0x3a => x < 0,
                    0x3b => x >= 0,
                    0x3c => x > 0,
                    _ => x <= 0,
                };
                if t {
                    f.pc = (pc as i64 + u1 as u16 as i16 as i64) as usize;
                    return Ok(None);
                }
                len = 2;
            }
            0x44..=0x4a => {
                let (bb, cc) = ((u1 & 0xff) as usize, (u1 >> 8) as usize);
                let v = self.array_get(r!(bb), r!(cc) as i32)?;
                match op {
                    0x45 => ww(f, aa, v),
                    0x47 => r!(aa) = (v & 1) as u32,
                    0x48 => r!(aa) = v as u8 as i8 as i32 as u32,
                    0x49 => r!(aa) = v as u16 as u32,
                    0x4a => r!(aa) = v as u16 as i16 as i32 as u32,
                    _ => r!(aa) = v as u32,
                }
                len = 2;
            }
            0x4b..=0x51 => {
                let (bb, cc) = ((u1 & 0xff) as usize, (u1 >> 8) as usize);
                let v = if op == 0x4c { rw(f, aa) } else { r!(aa) as u64 };
                let v = match op {
                    0x4e => v & 1,
                    0x4f => v as u8 as u64,
                    0x50 | 0x51 => v as u16 as u64,
                    _ => v,
                };
                self.array_set(r!(bb), r!(cc) as i32, v)?;
                len = 2;
            }
            0x52..=0x58 => {
                let slot = self.inst_slot(m.dex, u1)?;
                let o = r!(b);
                if o == 0 {
                    return Err(self.npe());
                }
                let v = self.heap[o as usize].fields.get(slot).copied().unwrap_or(0);
                if op == 0x53 {
                    ww(f, a, v);
                } else {
                    r!(a) = v as u32;
                }
                len = 2;
            }
            0x59..=0x5f => {
                let slot = self.inst_slot(m.dex, u1)?;
                let o = r!(b);
                if o == 0 {
                    return Err(self.npe());
                }
                let v = if op == 0x5a { rw(f, a) } else { r!(a) as u64 };
                let fl = &mut self.heap[o as usize].fields;
                if slot >= fl.len() {
                    fl.resize(slot + 1, 0);
                }
                fl[slot] = v;

                len = 2;
            }
            0x60..=0x66 => {
                let (c, s) = self.static_slot(m.dex, u1)?;
                let v = self.classes[c].statics[s];
                if op == 0x61 {
                    ww(f, aa, v);
                } else {
                    r!(aa) = v as u32;
                }
                len = 2;
            }
            0x67..=0x6d => {
                let (c, s) = self.static_slot(m.dex, u1)?;
                let v = if op == 0x68 { rw(f, aa) } else { r!(aa) as u64 };
                self.classes[c].statics[s] = v;
                len = 2;
            }
            0x6e..=0x72 => {
                let cnt = b;
                let all = [(u2 & 0xf) as usize, ((u2 >> 4) & 0xf) as usize, ((u2 >> 8) & 0xf) as usize, ((u2 >> 12) & 0xf) as usize, a];
                let args: Vec<u32> = all[..cnt].iter().map(|&x| f.regs[x]).collect();
                self.invoke_site(m, f, op, u1, args)?;
                len = 3;
            }
            0x74..=0x78 => {
                let args: Vec<u32> = (0..aa).map(|k| f.regs[u2 as usize + k]).collect();
                self.invoke_site(m, f, op - 6, u1, args)?;
                len = 3;
            }
            0x7b..=0x8f => {
                match op {
                    0x7b => r!(a) = (r!(b) as i32).wrapping_neg() as u32,
                    0x7c => r!(a) = !r!(b),
                    0x7d => {
                        let v = (rw(f, b) as i64).wrapping_neg() as u64;
                        ww(f, a, v);
                    }
                    0x7e => {
                        let v = !rw(f, b);
                        ww(f, a, v);
                    }
                    0x7f => r!(a) = (-f32::from_bits(r!(b))).to_bits(),
                    0x80 => {
                        let v = (-f64::from_bits(rw(f, b))).to_bits();
                        ww(f, a, v);
                    }
                    0x81 => ww(f, a, r!(b) as i32 as i64 as u64),
                    0x82 => r!(a) = (r!(b) as i32 as f32).to_bits(),
                    0x83 => ww(f, a, (r!(b) as i32 as f64).to_bits()),
                    0x84 => r!(a) = rw(f, b) as u32,
                    0x85 => r!(a) = (rw(f, b) as i64 as f32).to_bits(),
                    0x86 => {
                        let v = (rw(f, b) as i64 as f64).to_bits();
                        ww(f, a, v);
                    }
                    0x87 => r!(a) = (f32::from_bits(r!(b)) as i32) as u32,
                    0x88 => ww(f, a, (f32::from_bits(r!(b)) as i64) as u64),
                    0x89 => ww(f, a, (f32::from_bits(r!(b)) as f64).to_bits()),
                    0x8a => r!(a) = (f64::from_bits(rw(f, b)) as i32) as u32,
                    0x8b => {
                        let v = (f64::from_bits(rw(f, b)) as i64) as u64;
                        ww(f, a, v);
                    }
                    0x8c => r!(a) = (f64::from_bits(rw(f, b)) as f32).to_bits(),
                    0x8d => r!(a) = r!(b) as u8 as i8 as i32 as u32,
                    0x8e => r!(a) = r!(b) as u16 as u32,
                    _ => r!(a) = r!(b) as u16 as i16 as i32 as u32,
                }
            }
            0x90..=0xcf => {
                let two = op >= 0xb0;
                let base = if two { op - 0x20 } else { op };
                let (dst, xr, yr) = if two { (a, a, b) } else { (aa, (u1 & 0xff) as usize, (u1 >> 8) as usize) };
                let wide_x = matches!(base, 0x9b..=0xa5 | 0xab..=0xaf);
                let wide_y = wide_x && !matches!(base, 0xa3..=0xa5);
                let x = if wide_x { rw(f, xr) } else { r!(xr) as u64 };
                let y = if wide_y { rw(f, yr) } else { r!(yr) as u64 };
                let res = self.binop(base, x, y)?;
                if wide_x {
                    ww(f, dst, res);
                } else {
                    r!(dst) = res as u32;
                }
                if !two {
                    len = 2;
                }
            }
            0xd0..=0xd7 => {
                let x = r!(b) as i32;
                let y = u1 as u16 as i16 as i32;
                let k = op - 0xd0;
                r!(a) = if k == 1 { self.int_op(1, y, x)? } else { self.int_op(k, x, y)? } as u32;
                len = 2;
            }
            0xd8..=0xe2 => {
                let x = r!(u1 & 0xff) as i32;
                let y = (u1 >> 8) as u8 as i8 as i32;
                let k = op - 0xd8;
                r!(aa) = if k == 1 { self.int_op(1, y, x)? } else { self.int_op(k, x, y)? } as u32;
                len = 2;
            }
            _ => {
                eprintln!("UNIMPLEMENTED opcode 0x{:02x} in {}", op, m.key);
                return Err(self.throw_new("Ljava/lang/UnsupportedOperationException;"));
            }
        }
        f.pc += len;
        Ok(None)
    }
}

impl Obj {
    pub(crate) fn kind_str(&self) -> Option<Vec<u16>> {
        match &self.kind {
            Kind::Str(u) => Some(u.clone()),
            _ => None,
        }
    }
}
