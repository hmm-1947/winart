use crate::runtime::{Kind, Obj, Ref, Runtime, R};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

type Res<T> = Result<T, (String, usize)>;

#[derive(Clone, Copy, PartialEq)]
enum Prop {
    Alpha,
    Upper,
    Lower,
    Digit,
    Alnum,
    Punct,
    Space,
    Blank,
    Cntrl,
    Graph,
    Print,
    XDigit,
    Ascii,
}

enum Item {
    Range(u32, u32),
    Digit(bool),
    Word(bool),
    Space(bool),
    HSpace(bool),
    VSpace(bool),
    Prop(Prop, bool),
    Sub(Cls),
}

struct Cls {
    neg: bool,
    items: Vec<Item>,
    and: Vec<Cls>,
}

enum Atom {
    Ch(u32),
    It(Item),
}

enum Node {
    Empty,
    Char(u32, bool),
    Any(bool, bool),
    Class(Box<Cls>, bool),
    Seq(Vec<Node>),
    Alt(Vec<Node>),
    Group(Box<Node>, usize),
    Rep(Box<Node>, usize, usize, u8),
    Bol(bool, bool),
    Eol(bool, bool),
    Begin,
    End,
    EndZ(bool),
    SearchStart,
    WordB(bool),
    Look(Box<Node>, bool, bool),
    Atomic(Box<Node>),
    Back(usize, bool),
}

fn ch(c: u32) -> Option<char> {
    char::from_u32(c)
}

fn lower(c: u32) -> u32 {
    match ch(c) {
        Some(x) => {
            let mut it = x.to_lowercase();
            let f = it.next().unwrap_or(x);
            if it.next().is_none() {
                f as u32
            } else {
                c
            }
        }
        None => c,
    }
}

fn upper(c: u32) -> u32 {
    match ch(c) {
        Some(x) => {
            let mut it = x.to_uppercase();
            let f = it.next().unwrap_or(x);
            if it.next().is_none() {
                f as u32
            } else {
                c
            }
        }
        None => c,
    }
}

fn feq(a: u32, b: u32, ic: bool) -> bool {
    a == b || (ic && (lower(a) == lower(b) || upper(a) == upper(b)))
}

fn is_digit(c: u32) -> bool {
    (0x30..=0x39).contains(&c) || (0x660..=0x669).contains(&c) || (0x6F0..=0x6F9).contains(&c) || (0x966..=0x96F).contains(&c) || (0xFF10..=0xFF19).contains(&c)
}

fn is_word(c: u32) -> bool {
    ch(c).map_or(false, |x| x.is_alphanumeric() || x == '_') || c == 0x200c || c == 0x200d
}

fn is_space(c: u32) -> bool {
    ch(c).map_or(false, |x| x.is_whitespace())
}

fn is_hspace(c: u32) -> bool {
    c == 32 || c == 9 || c == 0xA0 || c == 0x1680 || c == 0x180e || (0x2000..=0x200a).contains(&c) || c == 0x202f || c == 0x205f || c == 0x3000
}

fn is_vspace(c: u32) -> bool {
    (0x0A..=0x0D).contains(&c) || c == 0x85 || c == 0x2028 || c == 0x2029
}

fn is_term(c: u32, unix: bool) -> bool {
    if unix {
        c == 10
    } else {
        c == 10 || c == 13 || c == 0x85 || c == 0x2028 || c == 0x2029
    }
}

fn prop_match(p: Prop, c: u32) -> bool {
    let x = match ch(c) {
        Some(x) => x,
        None => return false,
    };
    match p {
        Prop::Alpha => x.is_alphabetic(),
        Prop::Upper => x.is_uppercase(),
        Prop::Lower => x.is_lowercase(),
        Prop::Digit => is_digit(c),
        Prop::Alnum => x.is_alphabetic() || is_digit(c),
        Prop::Punct => {
            x.is_ascii_punctuation() || (0x2010..=0x2027).contains(&c) || (0x3001..=0x3003).contains(&c) || matches!(c, 0xA1 | 0xA7 | 0xAB | 0xB6 | 0xB7 | 0xBB | 0xBF)
        }
        Prop::Space => x.is_whitespace(),
        Prop::Blank => c == 32 || c == 9,
        Prop::Cntrl => x.is_control(),
        Prop::Graph => !x.is_whitespace() && !x.is_control(),
        Prop::Print => !x.is_control(),
        Prop::XDigit => x.is_ascii_hexdigit(),
        Prop::Ascii => c < 128,
    }
}

fn item_hit(it: &Item, c: u32, ic: bool) -> bool {
    match it {
        Item::Range(a, b) => {
            (*a <= c && c <= *b) || (ic && {
                let l = lower(c);
                let u = upper(c);
                (*a <= l && l <= *b) || (*a <= u && u <= *b)
            })
        }
        Item::Digit(n) => is_digit(c) != *n,
        Item::Word(n) => is_word(c) != *n,
        Item::Space(n) => is_space(c) != *n,
        Item::HSpace(n) => is_hspace(c) != *n,
        Item::VSpace(n) => is_vspace(c) != *n,
        Item::Prop(p, n) => prop_match(*p, c) != *n,
        Item::Sub(cl) => cls_hit(cl, c, ic),
    }
}

fn cls_hit(cl: &Cls, c: u32, ic: bool) -> bool {
    let h = cl.items.iter().any(|i| item_hit(i, c, ic)) && cl.and.iter().all(|a| cls_hit(a, c, ic));
    h != cl.neg
}

#[derive(Clone, Copy)]
struct Fl {
    ic: bool,
    ml: bool,
    dot: bool,
    cm: bool,
    unix: bool,
}

struct P {
    s: Vec<u32>,
    i: usize,
    ng: usize,
    names: HashMap<String, usize>,
    f: Fl,
}

fn hexv(c: u32) -> Option<u32> {
    ch(c).and_then(|x| x.to_digit(16))
}

impl P {
    fn peek(&self) -> Option<u32> {
        self.s.get(self.i).copied()
    }

    fn peek_is(&self, c: char) -> bool {
        self.peek() == Some(c as u32)
    }

    fn err<T>(&self, m: &str, at: usize) -> Res<T> {
        Err((m.to_string(), at))
    }

    fn skip_cm(&mut self) {
        if !self.f.cm {
            return;
        }
        loop {
            match self.peek() {
                Some(c) if ch(c).map_or(false, |x| x.is_whitespace()) => self.i += 1,
                Some(35) => {
                    while let Some(c) = self.peek() {
                        self.i += 1;
                        if c == 10 || c == 13 {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    fn num(&mut self) -> Option<usize> {
        let st = self.i;
        let mut v = 0usize;
        while let Some(d) = self.peek().and_then(ch).and_then(|x| x.to_digit(10)) {
            v = v.saturating_mul(10).saturating_add(d as usize);
            self.i += 1;
        }
        if self.i == st {
            None
        } else {
            Some(v)
        }
    }

    fn alt(&mut self) -> Res<Node> {
        let mut v = vec![self.seq()?];
        while self.peek_is('|') {
            self.i += 1;
            v.push(self.seq()?);
        }
        Ok(if v.len() == 1 { v.pop().unwrap() } else { Node::Alt(v) })
    }

    fn seq(&mut self) -> Res<Node> {
        let mut v = vec![];
        loop {
            self.skip_cm();
            match self.peek() {
                None => break,
                Some(c) if c == ')' as u32 || c == '|' as u32 => break,
                _ => {}
            }
            let a = self.atom()?;
            let a = self.quant(a)?;
            v.push(a);
        }
        Ok(match v.len() {
            0 => Node::Empty,
            1 => v.pop().unwrap(),
            _ => Node::Seq(v),
        })
    }

    fn quant(&mut self, a: Node) -> Res<Node> {
        self.skip_cm();
        let (min, max) = match self.peek().and_then(ch) {
            Some('*') => {
                self.i += 1;
                (0, usize::MAX)
            }
            Some('+') => {
                self.i += 1;
                (1, usize::MAX)
            }
            Some('?') => {
                self.i += 1;
                (0, 1)
            }
            Some('{') => {
                let st = self.i;
                self.i += 1;
                let mn = match self.num() {
                    Some(x) => x,
                    None => return self.err("Illegal repetition", st),
                };
                let mx = if self.peek_is(',') {
                    self.i += 1;
                    self.num().unwrap_or(usize::MAX)
                } else {
                    mn
                };
                if !self.peek_is('}') {
                    return self.err("Unclosed counted closure", self.i);
                }
                self.i += 1;
                if mx < mn {
                    return self.err("Illegal repetition range", st);
                }
                (mn, mx)
            }
            _ => return Ok(a),
        };
        let mode = if self.peek_is('?') {
            self.i += 1;
            1
        } else if self.peek_is('+') {
            self.i += 1;
            2
        } else {
            0
        };
        Ok(Node::Rep(Box::new(a), min, max, mode))
    }

    fn atom(&mut self) -> Res<Node> {
        let c = self.s[self.i];
        let at = self.i;
        let cc = ch(c);
        match cc {
            Some('(') => self.group(),
            Some('[') => {
                self.i += 1;
                let neg = if self.peek_is('^') {
                    self.i += 1;
                    true
                } else {
                    false
                };
                let cl = self.class_body(neg, at)?;
                Ok(Node::Class(Box::new(cl), self.f.ic))
            }
            Some('.') => {
                self.i += 1;
                Ok(Node::Any(self.f.dot, self.f.unix))
            }
            Some('^') => {
                self.i += 1;
                Ok(Node::Bol(self.f.ml, self.f.unix))
            }
            Some('$') => {
                self.i += 1;
                Ok(Node::Eol(self.f.ml, self.f.unix))
            }
            Some('\\') => self.escape(),
            Some('*') | Some('+') | Some('?') => self.err(&format!("Dangling meta character '{}'", cc.unwrap()), at),
            Some('{') => self.err("Illegal repetition", at),
            _ => {
                self.i += 1;
                Ok(Node::Char(c, self.f.ic))
            }
        }
    }

    fn group(&mut self) -> Res<Node> {
        let open = self.i;
        self.i += 1;
        let saved = self.f;
        let node: Node = if self.peek_is('?') {
            self.i += 1;
            let c = self.peek().and_then(ch);
            match c {
                Some(':') => {
                    self.i += 1;
                    self.alt()?
                }
                Some('=') | Some('!') => {
                    self.i += 1;
                    let b = self.alt()?;
                    Node::Look(Box::new(b), true, c == Some('!'))
                }
                Some('>') => {
                    self.i += 1;
                    let b = self.alt()?;
                    Node::Atomic(Box::new(b))
                }
                Some('<') => {
                    self.i += 1;
                    let c2 = self.peek().and_then(ch);
                    if c2 == Some('=') || c2 == Some('!') {
                        self.i += 1;
                        let b = self.alt()?;
                        Node::Look(Box::new(b), false, c2 == Some('!'))
                    } else {
                        let mut nm = String::new();
                        loop {
                            match self.peek().and_then(ch) {
                                Some('>') => {
                                    self.i += 1;
                                    break;
                                }
                                Some(x) if x.is_ascii_alphanumeric() => {
                                    nm.push(x);
                                    self.i += 1;
                                }
                                _ => return self.err("named capturing group is missing trailing '>'", self.i),
                            }
                        }
                        if nm.is_empty() || nm.chars().next().map_or(false, |x| x.is_ascii_digit()) {
                            return self.err("capturing group name does not start with a Latin letter", open);
                        }
                        if self.names.contains_key(&nm) {
                            return self.err(&format!("Named capturing group <{}> is already defined", nm), open);
                        }
                        self.ng += 1;
                        let idx = self.ng;
                        self.names.insert(nm, idx);
                        let b = self.alt()?;
                        Node::Group(Box::new(b), idx)
                    }
                }
                _ => {
                    let mut on = true;
                    let r: Node;
                    loop {
                        let fc = match self.peek().and_then(ch) {
                            Some(x) => x,
                            None => return self.err("Unknown inline modifier", self.i),
                        };
                        self.i += 1;
                        match fc {
                            'i' => self.f.ic = on,
                            'm' => self.f.ml = on,
                            's' => self.f.dot = on,
                            'x' => self.f.cm = on,
                            'd' => self.f.unix = on,
                            'u' | 'U' => {}
                            '-' => on = false,
                            ')' => return Ok(Node::Empty),
                            ':' => {
                                r = self.alt()?;
                                break;
                            }
                            _ => return self.err("Unknown inline modifier", self.i - 1),
                        }
                    }
                    r
                }
            }
        } else {
            self.ng += 1;
            let idx = self.ng;
            let b = self.alt()?;
            Node::Group(Box::new(b), idx)
        };
        if !self.peek_is(')') {
            return self.err("Unclosed group", open);
        }
        self.i += 1;
        self.f = saved;
        Ok(node)
    }

    fn hex_n(&mut self, n: usize) -> Res<u32> {
        let st = self.i;
        let mut v = 0u32;
        for _ in 0..n {
            match self.peek().and_then(hexv) {
                Some(d) => {
                    v = v * 16 + d;
                    self.i += 1;
                }
                None => return self.err("Illegal hexadecimal escape sequence", st),
            }
        }
        Ok(v)
    }

    fn esc_char(&mut self, c: char) -> Res<Option<u32>> {
        let at = self.i - 1;
        Ok(Some(match c {
            't' => 9,
            'n' => 10,
            'r' => 13,
            'f' => 12,
            'a' => 7,
            'e' => 27,
            '0' => {
                let mut v = 0u32;
                let mut n = 0;
                while n < 3 {
                    match self.peek().and_then(ch).and_then(|x| x.to_digit(8)) {
                        Some(d) if v * 8 + d <= 0o377 => {
                            v = v * 8 + d;
                            self.i += 1;
                            n += 1;
                        }
                        _ => break,
                    }
                }
                if n == 0 {
                    return self.err("Illegal octal escape sequence", at);
                }
                v
            }
            'x' => {
                if self.peek_is('{') {
                    self.i += 1;
                    let st = self.i;
                    let mut v = 0u32;
                    while let Some(d) = self.peek().and_then(hexv) {
                        v = v.wrapping_mul(16).wrapping_add(d);
                        self.i += 1;
                    }
                    if !self.peek_is('}') || self.i == st || v > 0x10FFFF {
                        return self.err("Illegal hexadecimal escape sequence", at);
                    }
                    self.i += 1;
                    v
                } else {
                    self.hex_n(2)?
                }
            }
            'u' => {
                let hi = self.hex_n(4)?;
                if (0xD800..0xDC00).contains(&hi) && self.peek_is('\\') && self.s.get(self.i + 1) == Some(&('u' as u32)) {
                    let save = self.i;
                    self.i += 2;
                    match self.hex_n(4) {
                        Ok(lo) if (0xDC00..0xE000).contains(&lo) => 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00),
                        _ => {
                            self.i = save;
                            hi
                        }
                    }
                } else {
                    hi
                }
            }
            'c' => match self.peek() {
                Some(x) => {
                    self.i += 1;
                    x ^ 64
                }
                None => return self.err("Illegal control escape sequence", at),
            },
            x if !x.is_ascii_alphanumeric() => x as u32,
            _ => return Ok(None),
        }))
    }

    fn prop(&mut self, neg: bool) -> Res<Item> {
        let at = self.i;
        let name: String = if self.peek_is('{') {
            self.i += 1;
            let mut n = String::new();
            loop {
                match self.peek().and_then(ch) {
                    Some('}') => {
                        self.i += 1;
                        break;
                    }
                    Some(x) => {
                        n.push(x);
                        self.i += 1;
                    }
                    None => return self.err("Unclosed character family", at),
                }
            }
            n
        } else {
            match self.peek().and_then(ch) {
                Some(x) => {
                    self.i += 1;
                    x.to_string()
                }
                None => return self.err("Illegal character family", at),
            }
        };
        let n = name.strip_prefix("Is").unwrap_or(&name);
        let p = match n {
            "L" | "Alpha" | "Alphabetic" | "Letter" | "javaLetter" | "javaAlphabetic" => Prop::Alpha,
            "Lu" | "Upper" | "Uppercase" | "javaUpperCase" => Prop::Upper,
            "Ll" | "Lower" | "Lowercase" | "javaLowerCase" => Prop::Lower,
            "N" | "Nd" | "Digit" | "javaDigit" => Prop::Digit,
            "Alnum" | "javaLetterOrDigit" => Prop::Alnum,
            "P" | "Punct" | "Punctuation" => Prop::Punct,
            "Z" | "Zs" | "Space" | "White_Space" | "javaWhitespace" | "javaSpaceChar" => Prop::Space,
            "Blank" => Prop::Blank,
            "Cntrl" | "Cc" => Prop::Cntrl,
            "Graph" => Prop::Graph,
            "Print" => Prop::Print,
            "XDigit" => Prop::XDigit,
            "ASCII" => Prop::Ascii,
            _ => return self.err(&format!("Unknown character property name {{{}}}", name), at),
        };
        Ok(Item::Prop(p, neg))
    }

    fn class_atom(&mut self) -> Res<Atom> {
        let c = self.s[self.i];
        self.i += 1;
        if c != '\\' as u32 {
            return Ok(Atom::Ch(c));
        }
        let at = self.i - 1;
        let e = match self.peek() {
            Some(e) => e,
            None => return self.err("Unclosed character class", at),
        };
        self.i += 1;
        let ec = match ch(e) {
            Some(x) => x,
            None => return Ok(Atom::Ch(e)),
        };
        Ok(match ec {
            'd' => Atom::It(Item::Digit(false)),
            'D' => Atom::It(Item::Digit(true)),
            'w' => Atom::It(Item::Word(false)),
            'W' => Atom::It(Item::Word(true)),
            's' => Atom::It(Item::Space(false)),
            'S' => Atom::It(Item::Space(true)),
            'h' => Atom::It(Item::HSpace(false)),
            'H' => Atom::It(Item::HSpace(true)),
            'v' => Atom::It(Item::VSpace(false)),
            'V' => Atom::It(Item::VSpace(true)),
            'p' => Atom::It(self.prop(false)?),
            'P' => Atom::It(self.prop(true)?),
            _ => match self.esc_char(ec)? {
                Some(v) => Atom::Ch(v),
                None => return self.err("Illegal/unsupported escape sequence", at + 1),
            },
        })
    }

    fn class_body(&mut self, neg: bool, open: usize) -> Res<Cls> {
        let mut items = Vec::new();
        let mut and = Vec::new();
        let mut first = true;
        loop {
            let c = match self.peek() {
                Some(c) => c,
                None => return self.err("Unclosed character class", open),
            };
            if c == ']' as u32 && !first {
                self.i += 1;
                break;
            }
            first = false;
            if c == '[' as u32 {
                let o = self.i;
                self.i += 1;
                let neg2 = if self.peek_is('^') {
                    self.i += 1;
                    true
                } else {
                    false
                };
                let sub = self.class_body(neg2, o)?;
                items.push(Item::Sub(sub));
                continue;
            }
            if c == '&' as u32 && self.s.get(self.i + 1) == Some(&('&' as u32)) {
                self.i += 2;
                let rhs = self.class_body(false, open)?;
                and.push(rhs);
                break;
            }
            let lo = match self.class_atom()? {
                Atom::It(it) => {
                    items.push(it);
                    continue;
                }
                Atom::Ch(x) => x,
            };
            if self.peek_is('-') && self.s.get(self.i + 1).map_or(false, |&n| n != ']' as u32 && n != '[' as u32) {
                self.i += 1;
                match self.class_atom()? {
                    Atom::Ch(hi) => {
                        if hi < lo {
                            return self.err("Illegal character range", self.i - 1);
                        }
                        items.push(Item::Range(lo, hi));
                    }
                    Atom::It(it) => {
                        items.push(Item::Range(lo, lo));
                        items.push(Item::Range('-' as u32, '-' as u32));
                        items.push(it);
                    }
                }
            } else {
                items.push(Item::Range(lo, lo));
            }
        }
        Ok(Cls { neg, items, and })
    }

    fn escape(&mut self) -> Res<Node> {
        let at = self.i;
        self.i += 1;
        let e = match self.peek() {
            Some(e) => e,
            None => return self.err("Unexpected internal error", at),
        };
        self.i += 1;
        let ic = self.f.ic;
        let ec = match ch(e) {
            Some(x) => x,
            None => return Ok(Node::Char(e, ic)),
        };
        let cls = |it: Item| Node::Class(Box::new(Cls { neg: false, items: vec![it], and: vec![] }), false);
        Ok(match ec {
            'd' => cls(Item::Digit(false)),
            'D' => cls(Item::Digit(true)),
            'w' => cls(Item::Word(false)),
            'W' => cls(Item::Word(true)),
            's' => cls(Item::Space(false)),
            'S' => cls(Item::Space(true)),
            'h' => cls(Item::HSpace(false)),
            'H' => cls(Item::HSpace(true)),
            'v' => cls(Item::VSpace(false)),
            'V' => cls(Item::VSpace(true)),
            'b' => Node::WordB(false),
            'B' => Node::WordB(true),
            'A' => Node::Begin,
            'z' => Node::End,
            'Z' => Node::EndZ(self.f.unix),
            'G' => Node::SearchStart,
            'R' => Node::Alt(vec![Node::Seq(vec![Node::Char(13, false), Node::Char(10, false)]), cls(Item::VSpace(false))]),
            'E' => Node::Empty,
            'Q' => {
                let mut v = vec![];
                loop {
                    match self.peek() {
                        None => break,
                        Some(x) if x == '\\' as u32 && self.s.get(self.i + 1) == Some(&('E' as u32)) => {
                            self.i += 2;
                            break;
                        }
                        Some(x) => {
                            v.push(Node::Char(x, ic));
                            self.i += 1;
                        }
                    }
                }
                Node::Seq(v)
            }
            'p' => {
                let it = self.prop(false)?;
                cls(it)
            }
            'P' => {
                let it = self.prop(true)?;
                cls(it)
            }
            'k' => {
                if !self.peek_is('<') {
                    return self.err("\\k is not followed by '<' for named capturing group", at);
                }
                self.i += 1;
                let mut nm = String::new();
                loop {
                    match self.peek().and_then(ch) {
                        Some('>') => {
                            self.i += 1;
                            break;
                        }
                        Some(x) => {
                            nm.push(x);
                            self.i += 1;
                        }
                        None => return self.err("named capturing group is missing trailing '>'", at),
                    }
                }
                match self.names.get(&nm) {
                    Some(&g) => Node::Back(g, ic),
                    None => return self.err(&format!("named capturing group <{}> does not exist", nm), at),
                }
            }
            '1'..='9' => {
                let mut n = e - '0' as u32;
                while let Some(d) = self.peek().and_then(ch).and_then(|x| x.to_digit(10)) {
                    let nn = n * 10 + d;
                    if nn as usize > self.ng {
                        break;
                    }
                    n = nn;
                    self.i += 1;
                }
                if n as usize > self.ng {
                    return self.err("Back reference to non-existent group", at);
                }
                Node::Back(n as usize, ic)
            }
            _ => match self.esc_char(ec)? {
                Some(v) => Node::Char(v, ic),
                None => return self.err("Illegal/unsupported escape sequence", at + 1),
            },
        })
    }
}

struct St<'a> {
    s: &'a [u16],
    from: usize,
    to: usize,
    lo: usize,
    hi: usize,
    caps: Vec<i32>,
    hit_end: bool,
    req_end: bool,
    anch: bool,
    trans: bool,
    sstart: usize,
}

fn cp_raw(s: &[u16], i: usize, hi: usize) -> Option<(u32, usize)> {
    if i >= hi || i >= s.len() {
        return None;
    }
    let u = s[i] as u32;
    if (0xD800..0xDC00).contains(&u) && i + 1 < hi && i + 1 < s.len() {
        let l = s[i + 1] as u32;
        if (0xDC00..0xE000).contains(&l) {
            return Some((0x10000 + ((u - 0xD800) << 10) + (l - 0xDC00), 2));
        }
    }
    Some((u, 1))
}

fn prev_raw(s: &[u16], i: usize, lo: usize) -> Option<u32> {
    if i <= lo || i > s.len() {
        return None;
    }
    let u = s[i - 1] as u32;
    if (0xDC00..0xE000).contains(&u) && i - 1 > lo {
        let h = s[i - 2] as u32;
        if (0xD800..0xDC00).contains(&h) {
            return Some(0x10000 + ((h - 0xD800) << 10) + (u - 0xDC00));
        }
    }
    Some(u)
}

fn nxt(st: &mut St, i: usize) -> Option<(u32, usize)> {
    if i >= st.hi {
        st.hit_end = true;
        return None;
    }
    cp_raw(st.s, i, st.hi)
}

fn is_single(n: &Node) -> bool {
    matches!(n, Node::Char(..) | Node::Any(..) | Node::Class(..))
}

pub struct Regex {
    node: Node,
    pub ngroups: usize,
    pub names: HashMap<String, usize>,
}

impl Regex {
    pub fn new(pat: &[u16], flags: i32) -> Res<Regex> {
        let mut cps = Vec::with_capacity(pat.len());
        let mut i = 0;
        while i < pat.len() {
            let u = pat[i] as u32;
            if (0xD800..0xDC00).contains(&u) && i + 1 < pat.len() && (0xDC00..0xE000).contains(&(pat[i + 1] as u32)) {
                cps.push(0x10000 + ((u - 0xD800) << 10) + (pat[i + 1] as u32 - 0xDC00));
                i += 2;
            } else {
                cps.push(u);
                i += 1;
            }
        }
        let mut p = P {
            s: cps,
            i: 0,
            ng: 0,
            names: HashMap::new(),
            f: Fl { ic: flags & 2 != 0, cm: flags & 4 != 0, ml: flags & 8 != 0, dot: flags & 32 != 0, unix: flags & 1 != 0 },
        };
        let node = p.alt()?;
        if p.i < p.s.len() {
            return Err(("Unmatched closing ')'".to_string(), p.i.saturating_sub(1)));
        }
        Ok(Regex { node, ngroups: p.ng, names: p.names })
    }

    fn one(&self, n: &Node, st: &mut St, i: usize) -> Option<usize> {
        let (c, l) = nxt(st, i)?;
        let ok = match n {
            Node::Char(x, ic) => feq(*x, c, *ic),
            Node::Any(dot, unix) => *dot || !is_term(c, *unix),
            Node::Class(cl, ic) => cls_hit(cl, c, *ic),
            _ => false,
        };
        if ok {
            Some(i + l)
        } else {
            None
        }
    }

    fn seq<'a>(&self, v: &[Node], i: usize, st: &mut St<'a>, k: &mut dyn FnMut(&mut St<'a>, usize) -> bool) -> bool {
        match v.split_first() {
            None => k(st, i),
            Some((h, t)) => {
                if t.is_empty() {
                    self.m(h, i, st, k)
                } else {
                    self.m(h, i, st, &mut |st, j| self.seq(t, j, st, &mut *k))
                }
            }
        }
    }

    fn rep_single<'a>(&self, b: &Node, min: usize, max: usize, mode: u8, i: usize, st: &mut St<'a>, k: &mut dyn FnMut(&mut St<'a>, usize) -> bool) -> bool {
        if mode == 1 {
            let mut cur = i;
            let mut cnt = 0usize;
            while cnt < min {
                match self.one(b, st, cur) {
                    Some(j) => {
                        cur = j;
                        cnt += 1;
                    }
                    None => return false,
                }
            }
            loop {
                if k(st, cur) {
                    return true;
                }
                if cnt >= max {
                    return false;
                }
                match self.one(b, st, cur) {
                    Some(j) => {
                        cur = j;
                        cnt += 1;
                    }
                    None => return false,
                }
            }
        }
        let mut ps = vec![i];
        while ps.len() - 1 < max {
            let last = *ps.last().unwrap();
            match self.one(b, st, last) {
                Some(j) => ps.push(j),
                None => break,
            }
        }
        if ps.len() - 1 < min {
            return false;
        }
        if mode == 2 {
            return k(st, *ps.last().unwrap());
        }
        for c in (min..ps.len()).rev() {
            if k(st, ps[c]) {
                return true;
            }
        }
        false
    }

    fn rep<'a>(&self, b: &Node, min: usize, max: usize, mode: u8, cnt: usize, i: usize, st: &mut St<'a>, k: &mut dyn FnMut(&mut St<'a>, usize) -> bool) -> bool {
        if mode == 1 {
            if cnt >= min && k(st, i) {
                return true;
            }
            if cnt < max {
                return self.m(b, i, st, &mut |st, j| if j == i && cnt >= min { false } else { self.rep(b, min, max, mode, cnt + 1, j, st, &mut *k) });
            }
            return false;
        }
        if cnt < max && self.m(b, i, st, &mut |st, j| if j == i && cnt >= min { false } else { self.rep(b, min, max, mode, cnt + 1, j, st, &mut *k) }) {
            return true;
        }
        cnt >= min && k(st, i)
    }

    fn m<'a>(&self, n: &Node, i: usize, st: &mut St<'a>, k: &mut dyn FnMut(&mut St<'a>, usize) -> bool) -> bool {
        match n {
            Node::Empty => k(st, i),
            Node::Char(..) | Node::Any(..) | Node::Class(..) => match self.one(n, st, i) {
                Some(j) => k(st, j),
                None => false,
            },
            Node::Seq(v) => self.seq(v, i, st, k),
            Node::Alt(v) => {
                for b in v {
                    if self.m(b, i, st, &mut *k) {
                        return true;
                    }
                }
                false
            }
            Node::Group(b, g) => {
                let g = *g;
                self.m(b, i, st, &mut |st, j| {
                    let o = (st.caps[2 * g], st.caps[2 * g + 1]);
                    st.caps[2 * g] = i as i32;
                    st.caps[2 * g + 1] = j as i32;
                    if k(st, j) {
                        true
                    } else {
                        st.caps[2 * g] = o.0;
                        st.caps[2 * g + 1] = o.1;
                        false
                    }
                })
            }
            Node::Rep(b, min, max, mode) => {
                if is_single(b) {
                    return self.rep_single(b, *min, *max, *mode, i, st, k);
                }
                if *mode == 2 {
                    let mut end = None;
                    if self.rep(b, *min, *max, 0, 0, i, st, &mut |_, j| {
                        end = Some(j);
                        true
                    }) {
                        return k(st, end.unwrap_or(i));
                    }
                    return false;
                }
                self.rep(b, *min, *max, *mode, 0, i, st, k)
            }
            Node::Atomic(b) => {
                let mut end = None;
                if self.m(b, i, st, &mut |_, j| {
                    end = Some(j);
                    true
                }) {
                    k(st, end.unwrap_or(i))
                } else {
                    false
                }
            }
            Node::Look(b, ahead, neg) => {
                let saved = (st.lo, st.hi);
                if st.trans {
                    st.lo = 0;
                    st.hi = st.s.len();
                }
                let caps_save = st.caps.clone();
                let ok = if *ahead {
                    self.m(b, i, st, &mut |_, _| true)
                } else {
                    let mut found = false;
                    let mut j = i as isize;
                    while j >= st.lo as isize {
                        if self.m(b, j as usize, st, &mut |_, e| e == i) {
                            found = true;
                            break;
                        }
                        j -= 1;
                    }
                    found
                };
                st.lo = saved.0;
                st.hi = saved.1;
                if *neg {
                    st.caps = caps_save;
                    if ok {
                        false
                    } else {
                        k(st, i)
                    }
                } else if ok {
                    if k(st, i) {
                        true
                    } else {
                        st.caps = caps_save;
                        false
                    }
                } else {
                    st.caps = caps_save;
                    false
                }
            }
            Node::Back(g, ic) => {
                let (s, e) = (st.caps[2 * g], st.caps[2 * g + 1]);
                if s < 0 || e < 0 {
                    return false;
                }
                let (s, e) = (s as usize, e as usize);
                let l = e - s;
                if i + l > st.hi {
                    st.hit_end = true;
                    return false;
                }
                for q in 0..l {
                    if !feq(st.s[s + q] as u32, st.s[i + q] as u32, *ic) {
                        return false;
                    }
                }
                k(st, i + l)
            }
            Node::Bol(ml, unix) => {
                let b = if st.anch { st.from } else { 0 };
                let aend = if st.anch { st.to } else { st.s.len() };
                let ok = if i == b {
                    true
                } else if !*ml || i > st.s.len() {
                    false
                } else {
                    let p = st.s[i - 1] as u32;
                    i < aend && is_term(p, *unix) && !(p == 13 && !*unix && st.s.get(i).map_or(false, |&x| x == 10))
                };
                if ok {
                    k(st, i)
                } else {
                    false
                }
            }
            Node::Eol(ml, unix) => {
                let e = if st.anch { st.to } else { st.s.len() };
                let ok = if i == e {
                    st.req_end = true;
                    true
                } else if i > e {
                    false
                } else if *ml {
                    let c = st.s[i] as u32;
                    is_term(c, *unix) && !(c == 10 && !*unix && i > 0 && st.s[i - 1] == 13)
                } else if i + 1 == e && is_term(st.s[i] as u32, *unix) {
                    st.req_end = true;
                    true
                } else if i + 2 == e && !*unix && st.s[i] == 13 && st.s[i + 1] == 10 {
                    st.req_end = true;
                    true
                } else {
                    false
                };
                if ok {
                    k(st, i)
                } else {
                    false
                }
            }
            Node::Begin => {
                if i == (if st.anch { st.from } else { 0 }) {
                    k(st, i)
                } else {
                    false
                }
            }
            Node::End => {
                if i == (if st.anch { st.to } else { st.s.len() }) {
                    st.req_end = true;
                    k(st, i)
                } else {
                    false
                }
            }
            Node::EndZ(unix) => {
                let e = if st.anch { st.to } else { st.s.len() };
                let ok = i == e || (i + 1 == e && is_term(st.s[i] as u32, *unix)) || (i + 2 == e && !*unix && st.s[i] == 13 && st.s[i + 1] == 10);
                if ok {
                    st.req_end = true;
                    k(st, i)
                } else {
                    false
                }
            }
            Node::SearchStart => {
                if i == st.sstart {
                    k(st, i)
                } else {
                    false
                }
            }
            Node::WordB(neg) => {
                let before = prev_raw(st.s, i, st.lo).map_or(false, is_word);
                let after = cp_raw(st.s, i, st.hi).map_or(false, |(c, _)| is_word(c));
                if i >= st.hi {
                    st.hit_end = true;
                }
                if (before != after) != *neg {
                    k(st, i)
                } else {
                    false
                }
            }
        }
    }

    fn try_at<'a>(&self, st: &mut St<'a>, s: usize, full: bool) -> bool {
        for c in st.caps.iter_mut() {
            *c = -1;
        }
        let to = st.to;
        self.m(&self.node, s, st, &mut |st, j| {
            if full && j != to {
                return false;
            }
            st.caps[0] = s as i32;
            st.caps[1] = j as i32;
            true
        })
    }
}

pub struct Matcher {
    re: Rc<Regex>,
    input: Vec<u16>,
    from: usize,
    to: usize,
    anch: bool,
    trans: bool,
    next: usize,
    prev_empty: bool,
    pub hit_end: bool,
    pub req_end: bool,
}

impl Matcher {
    pub fn new(re: Rc<Regex>) -> Matcher {
        Matcher { re, input: vec![], from: 0, to: 0, anch: true, trans: false, next: 0, prev_empty: false, hit_end: false, req_end: false }
    }

    pub fn groups(&self) -> usize {
        self.re.ngroups
    }

    pub fn set_input(&mut self, input: Vec<u16>, from: usize, to: usize) {
        let l = input.len();
        self.input = input;
        self.from = from.min(l);
        self.to = to.min(l).max(self.from);
        self.next = self.from;
        self.prev_empty = false;
        self.hit_end = false;
        self.req_end = false;
    }

    pub fn set_anchoring(&mut self, v: bool) {
        self.anch = v;
    }

    pub fn set_transparent(&mut self, v: bool) {
        self.trans = v;
    }

    fn run(&mut self, start: usize, full: bool, search: bool) -> Option<Vec<i32>> {
        let input = std::mem::take(&mut self.input);
        let re = self.re.clone();
        let (from, to) = (self.from, self.to);
        let mut st = St { s: &input, from, to, lo: from, hi: to, caps: vec![-1; 2 * (re.ngroups + 1)], hit_end: false, req_end: false, anch: self.anch, trans: self.trans, sstart: start };
        let mut found = None;
        let mut s = start;
        while s <= to {
            if re.try_at(&mut st, s, full) {
                found = Some(st.caps.clone());
                break;
            }
            if !search {
                break;
            }
            if s >= to {
                st.hit_end = true;
                break;
            }
            s += cp_raw(st.s, s, to).map_or(1, |(_, l)| l);
        }
        let (he, rq) = (st.hit_end, st.req_end);
        drop(st);
        self.input = input;
        self.hit_end = he;
        self.req_end = rq;
        match &found {
            Some(c) => {
                self.next = c[1] as usize;
                self.prev_empty = c[0] == c[1];
            }
            None => {
                self.next = self.to + 1;
            }
        }
        found
    }

    pub fn find_from(&mut self, start: usize) -> Option<Vec<i32>> {
        self.prev_empty = false;
        self.run(start, false, true)
    }

    pub fn find_next(&mut self) -> Option<Vec<i32>> {
        let mut start = self.next;
        if self.prev_empty {
            start += cp_raw(&self.input, start, self.to).map_or(1, |(_, l)| l);
        }
        self.run(start, false, true)
    }

    pub fn matches(&mut self) -> Option<Vec<i32>> {
        self.run(self.from, true, false)
    }

    pub fn looking_at(&mut self) -> Option<Vec<i32>> {
        self.run(self.from, false, false)
    }
}

fn lng(a: &[u32], i: usize) -> u64 {
    match (a.get(i), a.get(i + 1)) {
        (Some(&l), Some(&h)) => l as u64 | ((h as u64) << 32),
        (Some(&l), None) => l as u64,
        _ => u64::MAX,
    }
}

impl Runtime {
    fn str_units(&self, r: u32) -> Vec<u16> {
        match &self.heap[r as usize].kind {
            Kind::Str(u) => u.clone(),
            _ => vec![],
        }
    }

    fn write_offsets(&mut self, arr: u32, caps: &[i32]) {
        if arr == 0 {
            return;
        }
        if let Kind::Array(v) = &mut self.heap[arr as usize].kind {
            for (q, c) in caps.iter().enumerate() {
                if q < v.len() {
                    v[q] = *c as u32 as u64;
                }
            }
        }
    }

    fn throw_pse(&mut self, desc: &str, pat: u32, idx: usize) -> Ref {
        let e = self.throw_new("Ljava/util/regex/PatternSyntaxException;");
        let d = self.new_string(desc.encode_utf16().collect());
        let c = self.heap[e as usize].class;
        for (n, v) in [("desc", d as u64), ("pattern", pat as u64), ("index", idx as u64)] {
            if let Some(&s) = self.classes[c].inst_idx.get(n) {
                if s < self.heap[e as usize].fields.len() {
                    self.heap[e as usize].fields[s] = v;
                }
            }
        }
        e
    }

    pub(crate) fn pattern_native(&mut self, key: &str, a: &[u32]) -> Option<R<u64>> {
        match key {
            "compileImpl(Ljava/lang/String;I)J" => {
                if a[0] == 0 {
                    return Some(Err(self.npe()));
                }
                let pat = self.str_units(a[0]);
                match Regex::new(&pat, a[1] as i32) {
                    Ok(r) => {
                        self.re_pats.push(Rc::new(r));
                        Some(Ok(self.re_pats.len() as u64))
                    }
                    Err((d, idx)) => Some(Err(self.throw_pse(&d, a[0], idx))),
                }
            }
            "getNativeFinalizer()J" => Some(Ok(0)),
            "openMatcherImpl(J)J" => {
                let p = self.re_pats.get((lng(a, 0) as usize).wrapping_sub(1))?.clone();
                self.re_ms.push(Matcher::new(p));
                Some(Ok(self.re_ms.len() as u64))
            }
            "getMatchedGroupIndexImpl(JLjava/lang/String;)I" => {
                let p = self.re_pats.get((lng(a, 0) as usize).wrapping_sub(1))?.clone();
                let n = String::from_utf16_lossy(&self.str_units(a[2]));
                match p.names.get(&n) {
                    Some(&g) => Some(Ok(g as u64)),
                    None => {
                        let m = format!("No capturing group in the pattern with the name {}", n);
                        Some(Err(self.throw_msg("Ljava/lang/IllegalArgumentException;", &m)))
                    }
                }
            }
            _ => None,
        }
    }

    pub(crate) fn matcher_native(&mut self, key: &str, a: &[u32]) -> Option<R<u64>> {
        if key == "getNativeFinalizer()J" {
            return Some(Ok(0));
        }
        let idx = (lng(a, 0) as usize).wrapping_sub(1);
        if idx >= self.re_ms.len() {
            return None;
        }
        let (res, arr): (Option<Vec<i32>>, u32) = match key {
            "findImpl(JI[I)Z" => {
                let st = a[2] as i32;
                let r = if st < 0 { None } else { self.re_ms[idx].find_from(st as usize) };
                (r, a[3])
            }
            "findNextImpl(J[I)Z" => (self.re_ms[idx].find_next(), a[2]),
            "lookingAtImpl(J[I)Z" => (self.re_ms[idx].looking_at(), a[2]),
            "matchesImpl(J[I)Z" => (self.re_ms[idx].matches(), a[2]),
            "groupCountImpl(J)I" => return Some(Ok(self.re_ms[idx].groups() as u64)),
            "hitEndImpl(J)Z" => return Some(Ok(self.re_ms[idx].hit_end as u64)),
            "requireEndImpl(J)Z" => return Some(Ok(self.re_ms[idx].req_end as u64)),
            "setInputImpl(JLjava/lang/String;II)V" => {
                let u = self.str_units(a[2]);
                self.re_ms[idx].set_input(u, a[3] as usize, a[4] as usize);
                return Some(Ok(0));
            }
            "useAnchoringBoundsImpl(JZ)V" => {
                self.re_ms[idx].set_anchoring(a[2] & 1 != 0);
                return Some(Ok(0));
            }
            "useTransparentBoundsImpl(JZ)V" => {
                self.re_ms[idx].set_transparent(a[2] & 1 != 0);
                return Some(Ok(0));
            }
            _ => return None,
        };
        match res {
            Some(c) => {
                self.write_offsets(arr, &c);
                Some(Ok(1))
            }
            None => Some(Ok(0)),
        }
    }

    pub(crate) fn thread_native(&mut self, key: &str, a: &[u32]) -> Option<R<u64>> {
        match key {
            "currentThread()Ljava/lang/Thread;" => {
                if self.main_thread == 0 {
                    let c = self.load("Ljava/lang/Thread;");
                    let n = self.classes[c].inst_total;
                    let mut fields = vec![0u64; n];
                    let nm = self.str_ref("main") as u64;
                    if let Some(&s) = self.classes[c].inst_idx.get("name") {
                        fields[s] = nm;
                    }
                    if let Some(&s) = self.classes[c].inst_idx.get("priority") {
                        fields[s] = 5;
                    }
                    if let Some(&s) = self.classes[c].inst_idx.get("tid") {
                        fields[s] = 1;
                    }
                    let r = self.alloc(Obj { class: c, fields, kind: Kind::Plain });
                    self.add_root(r);
                    self.main_thread = r;
                }
                Some(Ok(self.main_thread as u64))
            }
            "interrupted()Z" | "isInterrupted()Z" | "isInterrupted(Z)Z" => Some(Ok(0)),
            "yield()V" => Some(Ok(0)),
            "sleep(Ljava/lang/Object;JI)V" => {
                std::thread::sleep(Duration::from_millis(lng(a, 1)));
                Some(Ok(0))
            }
            _ => None,
        }
    }

    pub(crate) fn registry_native(&mut self, key: &str) -> Option<R<u64>> {
        match key {
            "registerNativeAllocation(Ljava/lang/Object;J)Ljava/lang/Runnable;" => Some(Ok(0)),
            _ => None,
        }
    }
}
