use crate::axml::{parse_xml, XmlEv};
use crate::runtime::{Kind, Obj, Runtime, R};
use std::rc::Rc;

const P: &str = "Landroid/winrt/Native;->";

fn int(v: i32) -> u64 {
    v as u32 as u64
}

impl Runtime {
    fn arg_str(&self, r: u32) -> Option<String> {
        if r == 0 {
            None
        } else {
            Some(self.string_value(r))
        }
    }

    fn ret_str(&mut self, s: Option<String>) -> u64 {
        match s {
            Some(s) => self.new_string(s.encode_utf16().collect()) as u64,
            None => 0,
        }
    }

    fn load_xml(&mut self, id: u32) -> Option<Rc<Vec<XmlEv>>> {
        if let Some(d) = self.xml_docs.get(&id) {
            return Some(d.clone());
        }
        let arsc = self.arsc.as_ref()?;
        let path = arsc.string(arsc.lookup(id)?)?;
        let data = self.apk.as_ref()?.read(&path)?;
        let doc = Rc::new(parse_xml(&data).ok()?);
        self.xml_docs.insert(id, doc.clone());
        Some(doc)
    }

    fn cursor(&self, h: u32) -> Option<(Rc<Vec<XmlEv>>, usize)> {
        self.xml_cur.get(h as usize).and_then(|c| c.clone())
    }

    fn create_view(&mut self, a: &[u32]) -> R<u64> {
        let name = match self.arg_str(a[0]) {
            Some(n) => n,
            None => return Ok(0),
        };
        let desc = if name.contains('.') {
            format!("L{};", name.replace('.', "/"))
        } else {
            let w = format!("Landroid/widget/{};", name);
            if self.defs.contains_key(&w) {
                w
            } else {
                format!("Landroid/view/{};", name)
            }
        };
        if !self.defs.contains_key(&desc) {
            return Ok(0);
        }
        let c = self.load(&desc);
        self.ensure_init(c)?;
        let two = self.classes[c].methods.get("<init>(Landroid/content/Context;Landroid/util/AttributeSet;)V").cloned();
        let one = self.classes[c].methods.get("<init>(Landroid/content/Context;)V").cloned();
        let n = self.classes[c].inst_total;
        let o = self.alloc(Obj { class: c, fields: vec![0; n], kind: Kind::Plain });
        if let Some(m) = two {
            self.invoke(&m, vec![o, a[1], a[2]])?;
        } else if let Some(m) = one {
            self.invoke(&m, vec![o, a[1]])?;
        } else {
            return Ok(0);
        }
        Ok(o as u64)
    }

    fn box_value(&self, r: u32) -> Option<(String, u64)> {
        let o = &self.heap[r as usize];
        let c = &self.classes[o.class];
        let slot = *c.inst_idx.get("value")?;
        o.fields.get(slot).map(|v| (c.name.clone(), *v))
    }

    fn format_args(&self, fmt: &str, args: &[u32]) -> String {
        let f: Vec<char> = fmt.chars().collect();
        let mut out = String::new();
        let mut next = 0usize;
        let mut i = 0;
        while i < f.len() {
            if f[i] != '%' || i + 1 >= f.len() {
                out.push(f[i]);
                i += 1;
                continue;
            }
            let mut j = i + 1;
            let mut num = 0usize;
            let mut digits = false;
            while j < f.len() && f[j].is_ascii_digit() {
                num = num * 10 + f[j].to_digit(10).unwrap() as usize;
                digits = true;
                j += 1;
            }
            let mut index = None;
            if digits && j < f.len() && f[j] == '$' {
                index = Some(num.saturating_sub(1));
                j += 1;
            }
            let mut prec = None;
            if j < f.len() && f[j] == '.' {
                j += 1;
                let mut p = 0usize;
                while j < f.len() && f[j].is_ascii_digit() {
                    p = p * 10 + f[j].to_digit(10).unwrap() as usize;
                    j += 1;
                }
                prec = Some(p);
            }

            if j >= f.len() {
                out.push(f[i]);
                i += 1;
                continue;
            }
            let conv = f[j];
            i = j + 1;
            if conv == '%' {
                out.push('%');
                continue;
            }
            if conv == 'n' {
                out.push('\n');
                continue;
            }
            let k = match index {
                Some(k) => k,
                None => {
                    next += 1;
                    next - 1
                }
            };
            let arg = args.get(k).copied().unwrap_or(0);
            if arg == 0 {
                out.push_str("null");
                continue;
            }
            if let Some(s) = self.heap[arg as usize].kind_str() {
                out.push_str(&String::from_utf16_lossy(&s));
                continue;
            }
            match self.box_value(arg) {
                Some((cn, v)) => match (conv, cn.as_str()) {
                    ('x', _) | ('X', _) => out.push_str(&format!("{:x}", if cn.ends_with("Long;") { v } else { v as u32 as u64 })),
                    ('c', _) => out.push(char::from_u32(v as u32).unwrap_or('?')),
                    (_, "Ljava/lang/Float;") => out.push_str(&format!("{:.*}", prec.unwrap_or(6), f32::from_bits(v as u32))),
                    (_, "Ljava/lang/Double;") => out.push_str(&format!("{:.*}", prec.unwrap_or(6), f64::from_bits(v))),
                    (_, "Ljava/lang/Long;") => out.push_str(&(v as i64).to_string()),
                    (_, "Ljava/lang/Boolean;") => out.push_str(if v & 1 != 0 { "true" } else { "false" }),
                    (_, "Ljava/lang/Character;") => out.push(char::from_u32(v as u32).unwrap_or('?')),
                    _ => out.push_str(&(v as u32 as i32).to_string()),
                },
                None => out.push_str("null"),
            }
        }
        out
    }

    pub(crate) fn res_native(&mut self, key: &str, a: &[u32]) -> Option<R<u64>> {

        let k = key.strip_prefix(P)?;
        Some(match k {
            "resType(I)I" => Ok(match self.arsc.as_ref().and_then(|r| r.lookup(a[0])) {
                Some(v) => v.ty as u64,
                None => int(-1),
            }),
            "resData(I)I" => Ok(self.arsc.as_ref().and_then(|r| r.lookup(a[0])).map(|v| v.data as u64).unwrap_or(0)),
            "resString(I)Ljava/lang/String;" => {
                let s = self.arsc.as_ref().and_then(|r| r.lookup(a[0]).and_then(|v| r.string(v)));
                Ok(self.ret_str(s))
            }
            "resId(Ljava/lang/String;Ljava/lang/String;)I" => {
                let (n, t) = (self.arg_str(a[0]).unwrap_or_default(), self.arg_str(a[1]).unwrap_or_default());
                Ok(match self.arsc.as_ref().and_then(|r| r.find(&n, &t)) {
                    Some(id) => id as u64,
                    None => 0,
                })
            }
            "format(Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/String;" => {
                let fmt = self.arg_str(a[0]).unwrap_or_default();
                let items: Vec<u32> = if a[1] == 0 {
                    vec![]
                } else {
                    match &self.heap[a[1] as usize].kind {
                        Kind::Array(v) => v.iter().map(|x| *x as u32).collect(),
                        _ => vec![],
                    }
                };
                let s = self.format_args(&fmt, &items);
                Ok(self.ret_str(Some(s)))
            }
            "dumpStr(Ljava/lang/String;)V" => {
                let s = self.arg_str(a[0]).unwrap_or_default();
                self.dump_buf.push_str(&s);
                Ok(0)
            }
            "dumpInt(I)V" => {
                self.dump_buf.push_str(&(a[0] as i32).to_string());
                Ok(0)
            }
            "dumpHex(I)V" => {
                self.dump_buf.push_str(&format!("{:x}", a[0]));
                Ok(0)
            }
            "dumpTake()Ljava/lang/String;" => {
                let s = std::mem::take(&mut self.dump_buf);
                Ok(self.ret_str(Some(s)))
            }
            "classExists(Ljava/lang/String;)Z" => {
                let d = format!("L{};", self.arg_str(a[0]).unwrap_or_default().replace('.', "/"));
                Ok(self.defs.contains_key(&d) as u64)
            }
            "createView(Ljava/lang/String;Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;" => self.create_view(a),
            "invokeClick(Ljava/lang/Object;Ljava/lang/String;Ljava/lang/Object;)Z" => {
                if a[0] == 0 {
                    return Some(Ok(0));
                }
                let name = self.arg_str(a[1]).unwrap_or_default();
                let class = self.heap[a[0] as usize].class;
                match self.find_method(class, &format!("{}(Landroid/view/View;)V", name)) {
                    Some(m) => self.invoke(&m, vec![a[0], a[2]]).map(|_| 1),
                    None => Ok(0),
                }
            }
            "xmlOpen(I)I" => Ok(match self.load_xml(a[0]) {
                Some(doc) => {
                    let slot = self.xml_cur.iter().position(|c| c.is_none());
                    match slot {
                        Some(i) => {
                            self.xml_cur[i] = Some((doc, 0));
                            i as u64
                        }
                        None => {
                            self.xml_cur.push(Some((doc, 0)));
                            (self.xml_cur.len() - 1) as u64
                        }
                    }
                }
                None => int(-1),
            }),
            "xmlClose(I)V" => {
                if let Some(c) = self.xml_cur.get_mut(a[0] as usize) {
                    *c = None;
                }
                Ok(0)
            }
            "xmlNext(I)I" => Ok(match self.xml_cur.get_mut(a[0] as usize) {
                Some(Some((doc, pos))) => {
                    if *pos >= doc.len() {
                        1
                    } else {
                        let code = match &doc[*pos] {
                            XmlEv::Start(..) => 2,
                            XmlEv::End(..) => 3,
                        };
                        *pos += 1;
                        code
                    }
                }
                _ => 1,
            }),
            "xmlName(I)Ljava/lang/String;" => {
                let s = self.cursor(a[0]).and_then(|(d, p)| match d.get(p.wrapping_sub(1)) {
                    Some(XmlEv::Start(n, _)) | Some(XmlEv::End(n)) => Some(n.clone()),
                    None => None,
                });
                Ok(self.ret_str(s))
            }
            "xmlAttrCount(I)I" => Ok(self
                .cursor(a[0])
                .map(|(d, p)| match d.get(p.wrapping_sub(1)) {
                    Some(XmlEv::Start(_, at)) => at.len() as u64,
                    _ => 0,
                })
                .unwrap_or(0)),
            _ if k.starts_with("xmlAttr") => {
                let cur = self.cursor(a[0]);
                let at = cur.as_ref().and_then(|(d, p)| match d.get(p.wrapping_sub(1)) {
                    Some(XmlEv::Start(_, at)) => at.get(a[1] as usize),
                    _ => None,
                });
                match k {
                    "xmlAttrNs(II)Ljava/lang/String;" => {
                        let s = at.map(|x| x.ns.clone());
                        Ok(self.ret_str(s))
                    }
                    "xmlAttrName(II)Ljava/lang/String;" => {
                        let s = at.map(|x| x.name.clone());
                        Ok(self.ret_str(s))
                    }
                    "xmlAttrString(II)Ljava/lang/String;" => {
                        let s = at.and_then(|x| x.raw.clone());
                        Ok(self.ret_str(s))
                    }
                    "xmlAttrRes(II)I" => Ok(at.map(|x| x.res_id as u64).unwrap_or(0)),
                    "xmlAttrType(II)I" => Ok(at.map(|x| x.ty as u64).unwrap_or(0)),
                    "xmlAttrData(II)I" => Ok(at.map(|x| x.data as u64).unwrap_or(0)),
                    _ => return None,
                }
            }
            _ => return None,
        })
    }
}
