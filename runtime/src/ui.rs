use crate::runtime::Runtime;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

type Hwnd = isize;

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Rect {
    l: i32,
    t: i32,
    r: i32,
    b: i32,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Default)]
struct Size {
    cx: i32,
    cy: i32,
}

#[repr(C)]
struct Msg {
    hwnd: Hwnd,
    message: u32,
    wparam: usize,
    lparam: isize,
    time: u32,
    pt: Point,
}

#[repr(C)]
struct PaintStruct {
    hdc: isize,
    erase: i32,
    rc: Rect,
    restore: i32,
    inc: i32,
    reserved: [u8; 32],
}

type WndProc = unsafe extern "system" fn(Hwnd, u32, usize, isize) -> isize;

#[repr(C)]
struct WndClass {
    style: u32,
    proc_: WndProc,
    cls_extra: i32,
    wnd_extra: i32,
    instance: isize,
    icon: isize,
    cursor: isize,
    background: isize,
    menu: *const u16,
    class_name: *const u16,
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassW(c: *const WndClass) -> u16;
    fn CreateWindowExW(ex: u32, cls: *const u16, name: *const u16, style: u32, x: i32, y: i32, w: i32, h: i32, parent: Hwnd, menu: isize, inst: isize, param: *const u8) -> Hwnd;
    fn ShowWindow(h: Hwnd, cmd: i32) -> i32;
    fn UpdateWindow(h: Hwnd) -> i32;
    fn GetMessageW(m: *mut Msg, h: Hwnd, a: u32, b: u32) -> i32;
    fn TranslateMessage(m: *const Msg) -> i32;
    fn DispatchMessageW(m: *const Msg) -> isize;
    fn DefWindowProcW(h: Hwnd, m: u32, w: usize, l: isize) -> isize;
    fn PostQuitMessage(code: i32);
    fn BeginPaint(h: Hwnd, ps: *mut PaintStruct) -> isize;
    fn EndPaint(h: Hwnd, ps: *const PaintStruct) -> i32;
    fn InvalidateRect(h: Hwnd, r: *const Rect, erase: i32) -> i32;
    fn GetClientRect(h: Hwnd, r: *mut Rect) -> i32;
    fn SetWindowTextW(h: Hwnd, s: *const u16) -> i32;
    fn LoadCursorW(inst: isize, name: usize) -> isize;
    fn AdjustWindowRectExForDpi(r: *mut Rect, style: u32, menu: i32, ex: u32, dpi: u32) -> i32;
    fn SetProcessDpiAwarenessContext(v: isize) -> i32;
    fn GetDpiForSystem() -> u32;
    fn GetDpiForWindow(h: Hwnd) -> u32;
    fn SetWindowPos(h: Hwnd, after: Hwnd, x: i32, y: i32, w: i32, hh: i32, flags: u32) -> i32;
    fn FillRect(dc: isize, r: *const Rect, brush: isize) -> i32;
    fn GetDC(h: Hwnd) -> isize;
    fn ReleaseDC(h: Hwnd, dc: isize) -> i32;
    fn SetCapture(h: Hwnd) -> Hwnd;
    fn ReleaseCapture() -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateSolidBrush(c: u32) -> isize;
    fn DeleteObject(o: isize) -> i32;
    fn CreateCompatibleDC(dc: isize) -> isize;
    fn CreateCompatibleBitmap(dc: isize, w: i32, h: i32) -> isize;
    fn SelectObject(dc: isize, o: isize) -> isize;
    fn DeleteDC(dc: isize) -> i32;
    fn BitBlt(dst: isize, x: i32, y: i32, w: i32, h: i32, src: isize, sx: i32, sy: i32, rop: u32) -> i32;
    fn CreateFontW(h: i32, w: i32, e: i32, o: i32, wt: i32, i: u32, u: u32, s: u32, cs: u32, op: u32, cp: u32, q: u32, pf: u32, face: *const u16) -> isize;
    fn SetBkMode(dc: isize, m: i32) -> i32;
    fn SetTextColor(dc: isize, c: u32) -> u32;
    fn TextOutW(dc: isize, x: i32, y: i32, s: *const u16, n: i32) -> i32;
    fn GetTextExtentPoint32W(dc: isize, s: *const u16, n: i32, sz: *mut Size) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(n: *const u16) -> isize;
}

thread_local! {
    static RT: Cell<*mut Runtime> = Cell::new(std::ptr::null_mut());
    static WND: Cell<Hwnd> = Cell::new(0);
    static DC: Cell<isize> = Cell::new(0);
    static FONTS: RefCell<HashMap<i32, isize>> = RefCell::new(HashMap::new());
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn colorref(argb: i32) -> u32 {
    let c = argb as u32;
    ((c & 0xff) << 16) | (c & 0xff00) | ((c >> 16) & 0xff)
}

fn font(dc: isize, size: i32) {
    let f = FONTS.with(|m| {
        *m.borrow_mut().entry(size).or_insert_with(|| unsafe {
            let face = wide("Segoe UI");
            CreateFontW(-size, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr())
        })
    });
    unsafe {
        SelectObject(dc, f);
    }
}

pub fn fill_rect(x: i32, y: i32, w: i32, h: i32, color: i32) {
    let dc = DC.with(|d| d.get());
    if dc == 0 {
        return;
    }
    unsafe {
        let b = CreateSolidBrush(colorref(color));
        let r = Rect { l: x, t: y, r: x + w, b: y + h };
        FillRect(dc, &r, b);
        DeleteObject(b);
    }
}

pub fn stroke_rect(x: i32, y: i32, w: i32, h: i32, color: i32, t: i32) {
    fill_rect(x, y, w, t, color);
    fill_rect(x, y + h - t, w, t, color);
    fill_rect(x, y, t, h, color);
    fill_rect(x + w - t, y, t, h, color);
}

pub fn draw_text(s: &str, x: i32, y: i32, size: i32, color: i32) {
    let dc = DC.with(|d| d.get());
    if dc == 0 {
        return;
    }
    let u: Vec<u16> = s.encode_utf16().collect();
    font(dc, size);
    unsafe {
        SetBkMode(dc, 1);
        SetTextColor(dc, colorref(color));
        TextOutW(dc, x, y, u.as_ptr(), u.len() as i32);
    }
}

pub fn measure_text(s: &str, size: i32) -> i32 {
    let u: Vec<u16> = s.encode_utf16().collect();
    unsafe {
        let dc = GetDC(0);
        font(dc, size);
        let mut sz = Size::default();
        GetTextExtentPoint32W(dc, u.as_ptr(), u.len() as i32, &mut sz);
        ReleaseDC(0, dc);
        sz.cx
    }
}

pub fn invalidate() {
    let h = WND.with(|w| w.get());
    if h != 0 {
        unsafe {
            InvalidateRect(h, std::ptr::null(), 0);
        }
    }
}

pub fn set_title(s: &str) {
    let h = WND.with(|w| w.get());
    if h != 0 {
        let w = wide(s);
        unsafe {
            SetWindowTextW(h, w.as_ptr());
        }
    }
}

pub fn exit() {
    unsafe {
        PostQuitMessage(0);
    }
}

fn call(key: &str, args: Vec<u32>) {
    let rt = RT.with(|r| r.get());
    if rt.is_null() {
        return;
    }
    let rt = unsafe { &mut *rt };
    if let Err(e) = rt.call_static("Landroid/winrt/Host;", key, args) {
        eprintln!("exception in Host.{}: {}", key, rt.class_name_of(e));
    }
    rt.maybe_gc();
}

unsafe extern "system" fn wndproc(h: Hwnd, m: u32, w: usize, l: isize) -> isize {
    match m {
        0x000F => {
            let mut ps: PaintStruct = std::mem::zeroed();
            let hdc = BeginPaint(h, &mut ps);
            let mut rc = Rect::default();
            GetClientRect(h, &mut rc);
            let (cw, ch) = (rc.r - rc.l, rc.b - rc.t);
            if cw <= 0 || ch <= 0 {
                EndPaint(h, &ps);
                return 0;
            }
            let mem = CreateCompatibleDC(hdc);
            let bmp = CreateCompatibleBitmap(hdc, cw, ch);
            let old = SelectObject(mem, bmp);
            DC.with(|d| d.set(mem));
            call("draw(II)V", vec![cw as u32, ch as u32]);
            DC.with(|d| d.set(0));
            BitBlt(hdc, 0, 0, cw, ch, mem, 0, 0, 0x00CC0020);
            SelectObject(mem, old);
            DeleteObject(bmp);
            DeleteDC(mem);
            EndPaint(h, &ps);
            0
        }
        0x0005 => {
            InvalidateRect(h, std::ptr::null(), 0);
            0
        }
        0x02E0 => {
            let dpi = ((w >> 16) & 0xffff) as u32;
            let r = &*(l as *const Rect);
            SetWindowPos(h, 0, r.l, r.t, r.r - r.l, r.b - r.t, 0x0014);
            let d = density_for(dpi);
            call("init(F)V", vec![d.to_bits()]);
            InvalidateRect(h, std::ptr::null(), 0);
            0
        }
        0x0024 => {
            let dpi = GetDpiForWindow(h);
            let d = density_for(dpi);
            let mut r = Rect { l: 0, t: 0, r: (MIN_W_DP as f32 * d) as i32, b: (MIN_H_DP as f32 * d) as i32 };
            AdjustWindowRectExForDpi(&mut r, 0x00CF0000, 0, 0, dpi);
            let p = (l as *mut Point).add(3);
            (*p).x = r.r - r.l;
            (*p).y = r.b - r.t;
            0
        }
        0x0014 => 1,
        0x0201 | 0x0202 => {
            let x = (l & 0xffff) as u16 as i16 as i32;
            let y = ((l >> 16) & 0xffff) as u16 as i16 as i32;
            let down = m == 0x0201;
            if down {
                SetCapture(h);
            } else {
                ReleaseCapture();
            }
            call("touch(III)V", vec![if down { 0 } else { 1 }, x as u32, y as u32]);
            0
        }
        0x0002 => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(h, m, w, l),
    }
}

pub const DENSITY_SCALE: f32 = 1.25;
const MIN_W_DP: i32 = 220;
const MIN_H_DP: i32 = 300;

fn density_for(dpi: u32) -> f32 {
    dpi as f32 / 96.0 * DENSITY_SCALE
}

pub fn create(rt: &mut Runtime, title: &str, w_dp: i32, h_dp: i32) -> f32 {
    RT.with(|r| r.set(rt as *mut Runtime));
    unsafe {
        SetProcessDpiAwarenessContext(-4);
        let inst = GetModuleHandleW(std::ptr::null());
        let cls = wide("WinArtWindow");
        let wc = WndClass {
            style: 0x0003,
            proc_: wndproc,
            cls_extra: 0,
            wnd_extra: 0,
            instance: inst,
            icon: 0,
            cursor: LoadCursorW(0, 32512),
            background: 0,
            menu: std::ptr::null(),
            class_name: cls.as_ptr(),
        };
        RegisterClassW(&wc);
        let dpi = GetDpiForSystem();
        let d = density_for(dpi);
        let style = 0x00CF0000u32;
        let mut r = Rect { l: 0, t: 0, r: (w_dp as f32 * d) as i32, b: (h_dp as f32 * d) as i32 };
        AdjustWindowRectExForDpi(&mut r, style, 0, 0, dpi);
        let t = wide(title);
        let h = CreateWindowExW(0, cls.as_ptr(), t.as_ptr(), style, 100, 50, r.r - r.l, r.b - r.t, 0, 0, inst, std::ptr::null());
        WND.with(|w| w.set(h));
        let wd = density_for(GetDpiForWindow(h));
        wd
    }
}

pub fn pump() {
    unsafe {
        let h = WND.with(|w| w.get());
        ShowWindow(h, 5);
        UpdateWindow(h);
        let mut msg: Msg = std::mem::zeroed();
        while GetMessageW(&mut msg, 0, 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    RT.with(|r| r.set(std::ptr::null_mut()));
}
