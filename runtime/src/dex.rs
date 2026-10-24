use std::collections::HashMap;

pub const NO_INDEX: u32 = 0xffffffff;

pub struct Reader<'a> {
    pub d: &'a [u8],
    pub p: usize,
}

impl<'a> Reader<'a> {
    pub fn new(d: &'a [u8], p: usize) -> Self {
        Reader { d, p }
    }
    pub fn u8(&mut self) -> u8 {
        let v = self.d[self.p];
        self.p += 1;
        v
    }
    pub fn u16(&mut self) -> u16 {
        let v = u16::from_le_bytes([self.d[self.p], self.d[self.p + 1]]);
        self.p += 2;
        v
    }
    pub fn u32(&mut self) -> u32 {
        let v = u32::from_le_bytes([self.d[self.p], self.d[self.p + 1], self.d[self.p + 2], self.d[self.p + 3]]);
        self.p += 4;
        v
    }
    pub fn uleb(&mut self) -> u32 {
        let mut r = 0u32;
        let mut s = 0;
        loop {
            let b = self.u8();
            r |= ((b & 0x7f) as u32) << s;
            if b & 0x80 == 0 {
                break;
            }
            s += 7;
        }
        r
    }
    pub fn sleb(&mut self) -> i32 {
        let mut r: i32 = 0;
        let mut s = 0;
        let mut b;
        loop {
            b = self.u8();
            r |= ((b & 0x7f) as i32) << s;
            s += 7;
            if b & 0x80 == 0 {
                break;
            }
        }
        if s < 32 && (b & 0x40) != 0 {
            r |= -1i32 << s;
        }
        r
    }
}

pub struct Proto {
    pub ret: u16,
    pub params: Vec<u16>,
}

pub struct FieldId {
    pub class: u16,
    pub ty: u16,
    pub name: u32,
}

pub struct MethodId {
    pub class: u16,
    pub proto: u16,
    pub name: u32,
}

pub struct ClassDef {
    pub class: u32,
    pub access: u32,
    pub super_: u32,
    pub ifaces: Vec<u16>,
    pub class_data_off: u32,
    pub static_values_off: u32,
}

pub struct EncField {
    pub idx: u32,
    pub access: u32,
}

pub struct EncMethod {
    pub idx: u32,
    pub access: u32,
    pub code_off: u32,
}

#[derive(Default)]
pub struct ClassData {
    pub statics: Vec<EncField>,
    pub instances: Vec<EncField>,
    pub directs: Vec<EncMethod>,
    pub virtuals: Vec<EncMethod>,
}

pub struct Handler {
    pub catches: Vec<(u32, u32)>,
    pub catch_all: Option<u32>,
}

pub struct Try {
    pub start: u32,
    pub count: u32,
    pub handler: u32,
}

pub struct Code {
    pub regs: u16,
    pub ins: u16,
    pub outs: u16,
    pub insns: Vec<u16>,
    pub tries: Vec<Try>,
    pub handlers: HashMap<u32, Handler>,
}

#[derive(Clone, Debug)]
pub enum StaticVal {
    Int(u64),
    Str(u32),
    Null,
}

pub struct Dex {
    pub data: Vec<u8>,
    pub strings: Vec<String>,
    pub types: Vec<u32>,
    pub protos: Vec<Proto>,
    pub fields: Vec<FieldId>,
    pub methods: Vec<MethodId>,
    pub classes: Vec<ClassDef>,
}

fn rd32(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

fn rd16(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}

fn decode_mutf8(d: &[u8], mut p: usize) -> String {
    let mut units: Vec<u16> = Vec::new();
    while d[p] & 0x80 != 0 {
        p += 1;
    }
    p += 1;

    loop {
        let b = d[p];
        if b == 0 {
            break;
        }
        if b < 0x80 {
            units.push(b as u16);
            p += 1;
        } else if b & 0xe0 == 0xc0 {
            units.push((((b & 0x1f) as u16) << 6) | (d[p + 1] & 0x3f) as u16);
            p += 2;
        } else {
            units.push((((b & 0x0f) as u16) << 12) | (((d[p + 1] & 0x3f) as u16) << 6) | (d[p + 2] & 0x3f) as u16);
            p += 3;
        }
    }
    String::from_utf16_lossy(&units)
}

impl Dex {
    pub fn parse(data: Vec<u8>) -> Result<Dex, String> {
        if data.len() < 112 || &data[0..4] != b"dex\n" {
            return Err("bad dex magic".into());
        }
        let n_str = rd32(&data, 56) as usize;
        let o_str = rd32(&data, 60) as usize;
        let n_ty = rd32(&data, 64) as usize;
        let o_ty = rd32(&data, 68) as usize;
        let n_pr = rd32(&data, 72) as usize;
        let o_pr = rd32(&data, 76) as usize;
        let n_fl = rd32(&data, 80) as usize;
        let o_fl = rd32(&data, 84) as usize;
        let n_me = rd32(&data, 88) as usize;
        let o_me = rd32(&data, 92) as usize;
        let n_cl = rd32(&data, 96) as usize;
        let o_cl = rd32(&data, 100) as usize;
        let strings = (0..n_str).map(|i| decode_mutf8(&data, rd32(&data, o_str + i * 4) as usize)).collect();
        let types = (0..n_ty).map(|i| rd32(&data, o_ty + i * 4)).collect();
        let protos = (0..n_pr)
            .map(|i| {
                let b = o_pr + i * 12;
                let ret = rd32(&data, b + 4) as u16;
                let po = rd32(&data, b + 8) as usize;
                let mut params = Vec::new();
                if po != 0 {
                    let n = rd32(&data, po) as usize;
                    for k in 0..n {
                        params.push(rd16(&data, po + 4 + k * 2));
                    }
                }
                Proto { ret, params }
            })
            .collect();
        let fields = (0..n_fl)
            .map(|i| {
                let b = o_fl + i * 8;
                FieldId { class: rd16(&data, b), ty: rd16(&data, b + 2), name: rd32(&data, b + 4) }
            })
            .collect();
        let methods = (0..n_me)
            .map(|i| {
                let b = o_me + i * 8;
                MethodId { class: rd16(&data, b), proto: rd16(&data, b + 2), name: rd32(&data, b + 4) }
            })
            .collect();
        let classes = (0..n_cl)
            .map(|i| {
                let b = o_cl + i * 32;
                let io = rd32(&data, b + 12) as usize;
                let mut ifaces = Vec::new();
                if io != 0 {
                    let n = rd32(&data, io) as usize;
                    for k in 0..n {
                        ifaces.push(rd16(&data, io + 4 + k * 2));
                    }
                }
                ClassDef {
                    class: rd32(&data, b),
                    access: rd32(&data, b + 4),
                    super_: rd32(&data, b + 8),
                    ifaces,
                    class_data_off: rd32(&data, b + 24),
                    static_values_off: rd32(&data, b + 28),
                }
            })
            .collect();
        Ok(Dex { data, strings, types, protos, fields, methods, classes })
    }

    pub fn type_desc(&self, idx: u32) -> &str {
        &self.strings[self.types[idx as usize] as usize]
    }

    pub fn sig(&self, proto: u16) -> String {
        let p = &self.protos[proto as usize];
        let mut s = String::from("(");
        for t in &p.params {
            s.push_str(self.type_desc(*t as u32));
        }
        s.push(')');
        s.push_str(self.type_desc(p.ret as u32));
        s
    }

    pub fn method_name(&self, idx: u32) -> &str {
        &self.strings[self.methods[idx as usize].name as usize]
    }

    pub fn field_name(&self, idx: u32) -> &str {
        &self.strings[self.fields[idx as usize].name as usize]
    }

    pub fn class_data(&self, off: u32) -> ClassData {
        let mut cd = ClassData::default();
        if off == 0 {
            return cd;
        }
        let mut r = Reader::new(&self.data, off as usize);
        let ns = r.uleb();
        let ni = r.uleb();
        let nd = r.uleb();
        let nv = r.uleb();
        let mut idx = 0u32;
        for _ in 0..ns {
            idx += r.uleb();
            cd.statics.push(EncField { idx, access: r.uleb() });
        }
        idx = 0;
        for _ in 0..ni {
            idx += r.uleb();
            cd.instances.push(EncField { idx, access: r.uleb() });
        }
        idx = 0;
        for _ in 0..nd {
            idx += r.uleb();
            let access = r.uleb();
            let code_off = r.uleb();
            cd.directs.push(EncMethod { idx, access, code_off });
        }
        idx = 0;
        for _ in 0..nv {
            idx += r.uleb();
            let access = r.uleb();
            let code_off = r.uleb();
            cd.virtuals.push(EncMethod { idx, access, code_off });
        }
        cd
    }

    pub fn code(&self, off: u32) -> Code {
        let mut r = Reader::new(&self.data, off as usize);
        let regs = r.u16();
        let ins = r.u16();
        let outs = r.u16();
        let tries_size = r.u16() as usize;
        let _debug = r.u32();
        let n = r.u32() as usize;
        let mut insns = Vec::with_capacity(n);
        for _ in 0..n {
            insns.push(r.u16());
        }
        let mut tries = Vec::new();
        let mut handlers = HashMap::new();
        if tries_size > 0 {
            if n % 2 == 1 {
                r.u16();
            }
            for _ in 0..tries_size {
                let start = r.u32();
                let count = r.u16() as u32;
                let handler = r.u16() as u32;
                tries.push(Try { start, count, handler });
            }
            let base = r.p;
            let hn = r.uleb();
            for _ in 0..hn {
                let key = (r.p - base) as u32;
                let sz = r.sleb();
                let cnt = sz.unsigned_abs() as usize;
                let mut catches = Vec::new();
                for _ in 0..cnt {
                    let t = r.uleb();
                    let a = r.uleb();
                    catches.push((t, a));
                }
                let catch_all = if sz <= 0 { Some(r.uleb()) } else { None };
                handlers.insert(key, Handler { catches, catch_all });
            }
        }
        Code { regs, ins, outs, insns, tries, handlers }
    }

    pub fn static_values(&self, off: u32) -> Vec<StaticVal> {
        let mut out = Vec::new();
        if off == 0 {
            return out;
        }
        let mut r = Reader::new(&self.data, off as usize);
        let n = r.uleb();
        for _ in 0..n {
            let h = r.u8();
            let ty = h & 0x1f;
            let arg = (h >> 5) as usize;
            let v = match ty {
                0x1e => StaticVal::Null,
                0x1f => StaticVal::Int(arg as u64),
                0x1c | 0x1d | 0x19 | 0x1b | 0x1a => {
                    for _ in 0..=arg {
                        r.u8();
                    }
                    StaticVal::Null
                }
                _ => {
                    let size = arg + 1;
                    let mut raw: u64 = 0;
                    for k in 0..size {
                        raw |= (r.u8() as u64) << (8 * k);
                    }
                    match ty {
                        0x00 | 0x02 | 0x04 | 0x06 => {
                            let shift = 64 - 8 * size as u32;
                            StaticVal::Int((((raw << shift) as i64) >> shift) as u64)
                        }
                        0x10 => StaticVal::Int(raw << (8 * (4 - size))),
                        0x11 => StaticVal::Int(raw << (8 * (8 - size))),
                        0x17 => StaticVal::Str(raw as u32),
                        _ => StaticVal::Int(raw),
                    }
                }
            };
            out.push(v);
        }
        out
    }
}
