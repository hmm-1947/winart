use crate::runtime::{Kind, Obj, Ref, Runtime, R};
use std::cell::Cell;
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

struct Gil {
    held: bool,
    epoch: u64,
    gen: u64,
    waiting: u32,
}

static GIL: Mutex<Gil> = Mutex::new(Gil { held: true, epoch: 0, gen: 0, waiting: 0 });
static CV: Condvar = Condvar::new();
static NEXT_TID: AtomicU32 = AtomicU32::new(2);
static MAIN_WAKE: AtomicBool = AtomicBool::new(false);
static START: OnceLock<Instant> = OnceLock::new();

thread_local! {
    static TID: Cell<u32> = const { Cell::new(1) };
    static CUR: Cell<Ref> = const { Cell::new(0) };
    static DEPTH: Cell<usize> = const { Cell::new(0) };
}

const ST_RUNNABLE: u8 = 1;
const ST_BLOCKED: u8 = 2;
const ST_WAITING: u8 = 3;
const ST_TIMED: u8 = 4;

fn lock() -> MutexGuard<'static, Gil> {
    GIL.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn gil_release() {
    lock().held = false;
    CV.notify_all();
}

pub fn gil_acquire() {
    let mut g = lock();
    g.waiting += 1;
    g = CV.wait_while(g, |s| s.held).unwrap_or_else(|e| e.into_inner());
    g.waiting -= 1;
    g.held = true;
    g.gen += 1;
}

pub fn notify_event() {
    lock().epoch += 1;
    CV.notify_all();
}

#[inline(never)]
fn gil_wait(d: Duration) {
    let mut g = lock();
    let e0 = g.epoch;
    g.held = false;
    CV.notify_all();
    g = CV.wait_timeout_while(g, d, |s| s.epoch == e0).unwrap_or_else(|e| e.into_inner()).0;
    g.waiting += 1;
    g = CV.wait_while(g, |s| s.held).unwrap_or_else(|e| e.into_inner());
    g.waiting -= 1;
    g.held = true;
    g.gen += 1;
}

#[inline(never)]
fn gil_yield() {
    let mut g = lock();
    if g.waiting == 0 {
        return;
    }
    let gen0 = g.gen;
    g.held = false;
    CV.notify_all();
    g = CV.wait_timeout_while(g, Duration::from_millis(5), |s| s.gen == gen0).unwrap_or_else(|e| e.into_inner()).0;
    g.waiting += 1;
    g = CV.wait_while(g, |s| s.held).unwrap_or_else(|e| e.into_inner());
    g.waiting -= 1;
    g.held = true;
    g.gen += 1;
}

pub fn uptime_ms() -> u64 {
    START.get_or_init(Instant::now).elapsed().as_millis() as u64 + 1000
}

pub fn take_main_wake() -> bool {
    MAIN_WAKE.swap(false, Ordering::SeqCst)
}

pub fn wake_main() {
    MAIN_WAKE.store(true, Ordering::SeqCst);
    notify_event();
    crate::ui::wake();
}

pub fn depth() -> usize {
    DEPTH.with(|d| d.get())
}

pub fn depth_add(n: usize) {
    DEPTH.with(|d| d.set(d.get() + n));
}

pub fn depth_sub(n: usize) {
    DEPTH.with(|d| d.set(d.get().saturating_sub(n)));
}

pub fn cur_tid() -> u32 {
    tid()
}

fn tid() -> u32 {
    TID.with(|t| t.get())
}

fn lng(a: &[u32], i: usize) -> u64 {
    (a.get(i).copied().unwrap_or(0) as u64) | ((a.get(i + 1).copied().unwrap_or(0) as u64) << 32)
}

pub struct ThInfo {
    pub tid: u32,
    pub interrupted: bool,
    pub state: u8,
}

#[derive(Default)]
pub struct Threads {
    pub info: HashMap<Ref, ThInfo>,
    pub mons: HashMap<Ref, (u32, u32)>,
    pub waiters: HashMap<Ref, Vec<(u32, bool)>>,
    pub mt: bool,
    pub ticks: u64,
    pub in_clinit: u32,
    pub blockers: u32,
    pub started: u32,
    pub frames: Vec<(u32, usize)>,
}

struct SendPtr(*mut Runtime);
unsafe impl Send for SendPtr {}
impl SendPtr {
    fn get(&self) -> *mut Runtime {
        self.0
    }
}

fn thread_main(p: SendPtr, t: Ref, id: u32) {
    gil_acquire();
    TID.with(|x| x.set(id));
    CUR.with(|c| c.set(t));
    let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let rt = unsafe { &mut *p.get() };
        rt.run_thread(t);
    }));
    if res.is_err() {
        eprintln!("winart: internal panic in java thread {}", id);
        std::process::exit(101);
    }
    gil_release();
}

impl Runtime {
    fn set_field(&mut self, obj: Ref, name: &str, v: u64) {
        let c = self.heap[obj as usize].class;
        if let Some(&s) = self.classes[c].inst_idx.get(name) {
            let f = &mut self.heap[obj as usize].fields;
            if f.len() <= s {
                f.resize(s + 1, 0);
            }
            f[s] = v;
        }
    }

    fn get_field(&self, obj: Ref, name: &str) -> u64 {
        let c = self.heap[obj as usize].class;
        match self.classes[c].inst_idx.get(name) {
            Some(&s) => self.heap[obj as usize].fields.get(s).copied().unwrap_or(0),
            None => 0,
        }
    }

    fn make_main_thread(&mut self) {
        let c = self.load("Ljava/lang/Thread;");
        let n = self.classes[c].inst_total;
        let obj = self.alloc(Obj { class: c, fields: vec![0; n], kind: Kind::Plain });
        self.add_root(obj);
        self.main_thread = obj;
        self.th.info.insert(obj, ThInfo { tid: 1, interrupted: false, state: ST_RUNNABLE });
        self.set_field(obj, "nativePeer", 1);
        self.set_field(obj, "priority", 5);
        let nm = self.str_ref("main");
        self.set_field(obj, "name", nm as u64);
        let _ = self.ensure_init(c);
        let g = self.load("Ljava/lang/ThreadGroup;");
        let _ = self.ensure_init(g);
        let grp = match self.classes[g].static_idx.get("mainThreadGroup") {
            Some(&i) => self.classes[g].statics[i] as u32,
            None => 0,
        };
        if grp != 0 {
            if let Some(m) = self.classes[c].methods.get("<init>(Ljava/lang/ThreadGroup;Ljava/lang/String;IZ)V").cloned() {
                if let Err(e) = self.invoke(&m, vec![obj, grp, nm, 5, 0]) {
                    eprintln!("winart: main Thread init failed: {}", self.class_name_of(e));
                    self.set_field(obj, "group", grp as u64);
                }
            }
        }
        self.set_field(obj, "nativePeer", 1);
        self.set_field(obj, "started", 1);
    }

    pub(crate) fn cur_thread(&mut self) -> Ref {
        let c = CUR.with(|c| c.get());
        if c != 0 {
            return c;
        }
        if self.main_thread == 0 {
            self.make_main_thread();
        }
        self.main_thread
    }

    fn set_state(&mut self, s: u8) {
        let t = self.cur_thread();
        if let Some(i) = self.th.info.get_mut(&t) {
            i.state = s;
        }
    }

    fn interrupted_now(&mut self) -> bool {
        let t = self.cur_thread();
        self.th.info.get(&t).map_or(false, |i| i.interrupted)
    }

    fn clear_interrupt(&mut self) {
        let t = self.cur_thread();
        if let Some(i) = self.th.info.get_mut(&t) {
            i.interrupted = false;
        }
    }

    pub(crate) fn block_until(&mut self, deadline: Option<Instant>, state: u8, interruptible: bool, mut cond: impl FnMut(&mut Runtime) -> bool) -> Result<bool, ()> {
        self.set_state(state);
        self.th.blockers += 1;
        let r = loop {
            if cond(self) {
                break Ok(true);
            }
            if interruptible && self.interrupted_now() {
                break Err(());
            }
            let now = Instant::now();
            let mut wait = Duration::from_millis(50);
            if let Some(d) = deadline {
                if now >= d {
                    break Ok(false);
                }
                wait = wait.min(d - now);
            }
            std::hint::black_box(self as *mut Runtime);
            gil_wait(wait);
            std::hint::black_box(self as *mut Runtime);
        };
        self.th.blockers -= 1;
        self.set_state(ST_RUNNABLE);
        r
    }

    pub fn idle_wait(&mut self, ms: u64) {
        let dl = Instant::now() + Duration::from_millis(ms);
        let _ = self.block_until(Some(dl), ST_TIMED, false, |_| take_main_wake());
    }

    pub(crate) fn preempt(&mut self) {
        if !self.th.mt || self.th.in_clinit > 0 {
            return;
        }
        self.th.ticks += 1;
        if self.th.ticks & 0x3ff == 0 {
            self.yield_now();
        }
    }

    pub(crate) fn yield_now(&mut self) {
        if !self.th.mt {
            return;
        }
        std::hint::black_box(self as *mut Runtime);
        gil_yield();
        std::hint::black_box(self as *mut Runtime);
    }

    pub(crate) fn monitor_enter(&mut self, o: Ref) -> R<()> {
        let me = tid();
        match self.th.mons.get_mut(&o) {
            None => {
                self.th.mons.insert(o, (me, 1));
                return Ok(());
            }
            Some(m) if m.0 == me => {
                m.1 += 1;
                return Ok(());
            }
            _ => {}
        }
        let _ = self.block_until(None, ST_BLOCKED, false, |rt| !rt.th.mons.contains_key(&o));
        self.th.mons.insert(o, (me, 1));
        Ok(())
    }

    pub(crate) fn monitor_exit(&mut self, o: Ref) {
        let me = tid();
        let mut freed = false;
        if let Some(m) = self.th.mons.get_mut(&o) {
            if m.0 == me {
                m.1 -= 1;
                if m.1 == 0 {
                    freed = true;
                }
            }
        }
        if freed {
            self.th.mons.remove(&o);
            if self.th.blockers > 0 {
                notify_event();
            }
        }
    }

    pub(crate) fn obj_wait(&mut self, o: Ref, ms: u64, nanos: u32) -> R<()> {
        if o == 0 {
            return Err(self.npe());
        }
        if self.interrupted_now() {
            self.clear_interrupt();
            return Err(self.throw_msg("Ljava/lang/InterruptedException;", "interrupted"));
        }
        let me = tid();
        let mut saved = 0;
        if let Some(m) = self.th.mons.get(&o) {
            if m.0 == me {
                saved = m.1;
            }
        }
        if saved > 0 {
            self.th.mons.remove(&o);
            notify_event();
        }
        self.th.waiters.entry(o).or_default().push((me, false));
        let deadline = if ms == 0 && nanos == 0 { None } else { Some(Instant::now() + Duration::from_millis(ms) + Duration::from_nanos(nanos as u64)) };
        let st = if deadline.is_some() { ST_TIMED } else { ST_WAITING };
        let r = self.block_until(deadline, st, true, move |rt| rt.th.waiters.get(&o).map_or(true, |v| v.iter().any(|w| w.0 == me && w.1)));
        if let Some(v) = self.th.waiters.get_mut(&o) {
            v.retain(|w| w.0 != me);
            if v.is_empty() {
                self.th.waiters.remove(&o);
            }
        }
        if saved > 0 {
            let _ = self.block_until(None, ST_BLOCKED, false, |rt| !rt.th.mons.contains_key(&o));
            self.th.mons.insert(o, (me, saved));
        }
        if r.is_err() {
            self.clear_interrupt();
            return Err(self.throw_msg("Ljava/lang/InterruptedException;", "interrupted"));
        }
        Ok(())
    }

    pub(crate) fn obj_notify(&mut self, o: Ref, all: bool) {
        if let Some(v) = self.th.waiters.get_mut(&o) {
            for w in v.iter_mut() {
                if !w.1 {
                    w.1 = true;
                    if !all {
                        break;
                    }
                }
            }
            notify_event();
        }
    }

    fn thread_create(&mut self, t: Ref) -> R<()> {
        self.cur_thread();
        self.th.mt = true;
        let id = NEXT_TID.fetch_add(1, Ordering::SeqCst);
        self.th.info.insert(t, ThInfo { tid: id, interrupted: false, state: ST_RUNNABLE });
        self.set_field(t, "nativePeer", id as u64);
        self.add_root(t);
        self.th.started += 1;
        let p = SendPtr(self as *mut Runtime);
        let b = std::thread::Builder::new().stack_size(256 * 1024 * 1024).spawn(move || thread_main(p, t, id));
        if b.is_err() {
            self.th.info.remove(&t);
            self.set_field(t, "nativePeer", 0);
            self.remove_root(t);
            return Err(self.throw_msg("Ljava/lang/OutOfMemoryError;", "unable to create native thread"));
        }
        Ok(())
    }

    fn run_thread(&mut self, t: Ref) {
        if let Err(e) = self.call_virtual(t, "run()V", vec![]) {
            self.report_uncaught(t, e);
        }
        self.thread_exit(t);
    }

    fn report_uncaught(&mut self, t: Ref, e: Ref) {
        let nm = self.get_field(t, "name") as u32;
        let tn = if nm != 0 { self.string_value(nm) } else { String::new() };
        let c = self.class_name_of(e);
        let m = self.get_field(e, "detailMessage") as u32;
        let ms = if m != 0 { self.string_value(m) } else { String::new() };
        eprintln!("winart: uncaught exception in thread \"{}\": {} {}", tn, c, ms);
        let _ = self.call_virtual(t, "dispatchUncaughtException(Ljava/lang/Throwable;)V", vec![e]);
    }

    fn thread_exit(&mut self, t: Ref) {
        self.set_field(t, "nativePeer", 0);
        let me = tid();
        self.th.mons.retain(|_, m| m.0 != me);
        let lk = self.get_field(t, "lock") as u32;
        if lk != 0 {
            self.obj_notify(lk, true);
        }
        self.th.info.remove(&t);
        self.remove_root(t);
        notify_event();
    }

    pub(crate) fn thread_native(&mut self, key: &str, a: &[u32]) -> Option<R<u64>> {
        let this = a.first().copied().unwrap_or(0);
        let r: R<u64> = match key {
            "currentThread()Ljava/lang/Thread;" => Ok(self.cur_thread() as u64),
            "yield()V" => {
                self.yield_now();
                Ok(0)
            }
            "sleep(Ljava/lang/Object;JI)V" => {
                let ms = lng(a, 1);
                let ns = a.get(3).copied().unwrap_or(0) as u64;
                if ms == 0 && ns == 0 {
                    self.yield_now();
                    return Some(Ok(0));
                }
                let dl = Instant::now() + Duration::from_millis(ms) + Duration::from_nanos(ns);
                match self.block_until(Some(dl), ST_TIMED, true, |_| false) {
                    Err(()) => {
                        self.clear_interrupt();
                        Err(self.throw_msg("Ljava/lang/InterruptedException;", "sleep interrupted"))
                    }
                    Ok(_) => Ok(0),
                }
            }
            "nativeCreate(Ljava/lang/Thread;JZ)V" => self.thread_create(this).map(|_| 0),
            "interrupted()Z" => {
                let t = self.cur_thread();
                let was = self.th.info.get(&t).map_or(false, |i| i.interrupted);
                self.clear_interrupt();
                Ok(was as u64)
            }
            "isInterrupted()Z" => Ok(self.th.info.get(&this).map_or(false, |i| i.interrupted) as u64),
            "interrupt0()V" => {
                if let Some(i) = self.th.info.get_mut(&this) {
                    i.interrupted = true;
                }
                notify_event();
                Ok(0)
            }
            "nativeGetStatus(Z)I" => {
                let started = a.get(1).copied().unwrap_or(0) & 1 != 0;
                Ok(match self.th.info.get(&this) {
                    Some(i) => i.state as u64,
                    None => {
                        if started {
                            5
                        } else {
                            0
                        }
                    }
                })
            }
            "holdsLock(Ljava/lang/Object;)Z" => {
                let me = tid();
                Ok(self.th.mons.get(&this).map_or(false, |m| m.0 == me) as u64)
            }
            "setNativeName(Ljava/lang/String;)V" | "setPriority0(I)V" => Ok(0),
            _ => return None,
        };
        Some(r)
    }

    pub(crate) fn thread_api(&mut self, key: &str, a: &[u32]) -> Option<R<u64>> {
        let r: R<u64> = match key {
            "Landroid/winrt/Native;->wakeMain()V" => {
                wake_main();
                Ok(0)
            }
            "Landroid/winrt/Native;->log(Ljava/lang/String;)V" => {
                let s = self.string_value(a.first().copied().unwrap_or(0));
                eprintln!("[log] {}", s);
                Ok(0)
            }
            "Landroid/winrt/Native;->uptimeMillis()J" => Ok(uptime_ms()),
            "Landroid/winrt/Native;->isMainThread()Z" => Ok((tid() == 1) as u64),
            "Landroid/winrt/Native;->threadId()I" => Ok(tid() as u64),
            _ => return None,
        };
        Some(r)
    }
}
