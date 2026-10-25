# winandruntime

Run an unmodified APK on Windows x64 with no emulator, VM, WSL or Android OS.

## Layout

| Path | Content |
|---|---|
| `aosp/` | AOSP `android15-release` sources (art, libcore, libnativehelper, dalvik, system/{libbase,libziparchive,logging,core,unwinding}, external/{zlib,lz4,zstd,vixl,fmtlib,tinyxml2,expat,icu,googletest}) |
| `runtime/` | `winart`, the Windows-native runtime (Rust, MSVC target) |
| `testapp/` | WinCalc: calculator APK built with SDK tools (no permissions, no network, no GMS) |
| `tools/fetch_aosp.ps1` | repeatable source fetch |

## Pipeline and status

| Stage | Android/ART | winart | State |
|---|---|---|---|
| APK container | libziparchive | `zip.rs` (stored + deflate) | done |
| Manifest | aapt2 / PackageParser | `axml.rs` (binary XML, launcher activity, sdk, perms) | done |
| DEX parse | libdexfile | `dex.rs` (ids, class_data, code items, try/catch, static values) | done |
| Class load/link | ClassLinker | `runtime.rs` load/layout/init/`<clinit>` | done, single loader |
| Execution | interpreter | `runtime.rs` step(): all core Dalvik opcodes, exceptions | done |
| Core libs | core-oj, core-libart | `boot/core.dex` built from `aosp/libcore`; Rust natives for String fast paths, `Class.getPrimitiveClass`, `Native.*` | partial |
| Framework | framework.jar | `framework/src` (Activity, View, ViewGroup, TextView, Button, LinearLayout, Bundle, Host) built to `boot/framework.dex`, drawn through GDI in `ui.rs` | minimal |
| GC | runtime/gc | mark-sweep in `runtime.rs` (`gc`/`maybe_gc`), runs between top-level calls, roots = statics, interned strings, class objects, pinned activity | done |
| Window | - | `ui.rs`: per-monitor DPI v2, density = dpi/96*1.25, relayout on every resize, min size 220x300dp | done |
| JIT/AOT | optimizing compiler, dex2oat | none | not needed yet |
| Windows machine code | quick entrypoints, oat | none | later |

Verified: original `WinCalc.apk` runs on Windows with no emulator.
`winart test testapp/out/WinCalc.apk --expect 5 7 "*" 6 = + 8 = / 5 = C 9 - 4 =` clicks the real buttons through `Host.clickByText`, reads the last `TextView.setText` value and exits non-zero on mismatch. Extra flags: `--gc` collects after every click, `--repeat N` loops the sequence (heap stays flat with `--gc`). `WINART_LOGTEXT=1` logs every `setText`, `WINART_NOGC=1` disables automatic collection.
`winart press WinCalc.apk 7 * 6 = ...` drives the `Calc` class directly without any UI.

## Minimum ART needed (from `aosp/art`)

Required semantics, whether ported or reimplemented:

1. `libdexfile` (83 files): dex parsing, descriptors, code items. Portable C++.
2. `runtime/class_linker`, `mirror/`, `art_method`, `art_field`: class load, link, vtable/iftable, init.
3. `runtime/interpreter` (switch interpreter): fallback for everything, enough alone for a calculator.
4. `runtime/gc` + `runtime/mirror`: heap, object layout, roots. Not required for a first run (no collection).
5. `runtime/jni`, `libnativehelper`: native method binding for libcore (`Object`, `String`, `System`, `Class`, `Thread`, `Math`, `StrictMath`, array copy, atomics).
6. `runtime/thread`, `monitor`, `jni_env`: one thread first, locks no-op.
7. Boot classpath dex: `core-oj`, `core-libart`, `core-icu4j`, `conscrypt`, `okhttp`, `bouncycastle`, `apache-xml`. Built from `aosp/libcore` (Java), not from ART.
8. Not required for the first goal: `compiler/` (JIT/AOT), `dex2oat`, `oat`/`image`, `profile`, `jvmti`, `artd`, `odrefresh`, `libnativebridge`, `libnativeloader`, `perfetto_hprof`.

Dependencies of ART runtime that Windows lacks:

| Linux/bionic dependency | Windows replacement |
|---|---|
| `mmap/mprotect/madvise` | `VirtualAlloc/VirtualProtect` |
| `SIGSEGV` for null/stack-overflow checks | vectored exception handler (`sigchainlib` replaced) |
| `futex`, `pthread` | `WaitOnAddress`, `SRWLOCK`, `FlsAlloc` |
| `/proc/self/maps`, `prctl`, `ashmem` | drop or emulate |
| `dlopen` for JNI libs | `LoadLibrary` (test app has none) |
| system properties | in-process table |
| SysV x86-64 ABI in `quick_entrypoints_x86_64.S` | rewrite for Win64 (rcx/rdx/r8/r9, 32-byte shadow space, xmm6-15 callee-saved) when JIT/AOT is added |

## Decision

AOSP does not build on Windows (Soong/Linux host only) and ART host Windows support was removed upstream. Porting full ART means a clang-cl build of roughly 800 runtime files plus libbase/libartbase/libdexfile and a bionic shim layer, with no way to run the unit tests.

Chosen route: winart, an ART-semantics runtime written for Windows, with `aosp/art` as reference. Same input (unmodified APK, `classes.dex`, Dalvik bytecode), same class-linking and exception rules. Interpreter first, then an x64 JIT emitting Win64-ABI code.

## Next milestones

1. Build `core-oj`/`core-libart` from `aosp/libcore` with javac+d8 (no Soong) into `boot/*.dex`; add as boot loader; delete the Rust `native()` intrinsics one by one as real classes load; implement libcore natives in Rust.
2. Mark-sweep GC and real monitors.
3. Android framework shim: dex classes for `android.app.Activity`, `Context`, `Bundle`, `view.View`, `ViewGroup`, `widget.TextView/Button/LinearLayout`, with native methods backed by Win32/GDI (or Direct2D). `ActivityThread` equivalent calls `Activity.onCreate`, then runs a message loop.
4. `resources.arsc` + compiled XML layouts, for APKs that use `setContentView(R.layout...)`.
5. Input events from Win32 to `View.dispatchTouchEvent` and `OnClickListener`.
6. x64 JIT with Win64 ABI, SEH-based implicit checks.

## Tooling found on this machine

rustc 1.98.1 (msvc), VS2022 BuildTools MSVC 14.44, Windows SDK 10.0.26100, Android SDK (build-tools 36: aapt2, d8, dexdump; platforms 34 to 37), NDK 28.2 (clang), cmake 3.22.1, JDK 25, git, python 3.10. `rea` handles native binaries only through Ghidra (`GHIDRA_INSTALL_DIR` unset) and rejects APK/archive targets; it becomes useful for native `.so` analysis later.

## Resources and layouts

`arsc.rs` parses resources.arsc, `axml.rs` parses binary XML, `LayoutInflater` builds standard views from compiled layouts (View, TextView, Button, LinearLayout, include, merge, onClick, ids, margins, weights, gravity, dimens, colors, strings). `Activity.setContentView(int)` and `findViewById` work.
Verified with testapp2 (LayoutTest.apk, normal aapt2-built XML layout): `winart test testapp2/out/LayoutTest.apk --dump --seen "Taps: 2" Tap Tap` prints the laid-out view tree and clicks real buttons. WinCalc still passes.

## String model

`java.lang.String` objects are `Kind::Str(Vec<u16>)` heap objects (UTF-16 code units, no instance fields). Literals, interned strings and every runtime-created string use this one representation, so dex code and natives always see the same objects.

`StringBuilder`, `StringBuffer`, `AbstractStringBuilder`, `Integer.toString`, `Formatter` and `Pattern` are the real libcore classes from `boot/core.dex`. They reach strings only through the String API, which is native, so `+` concatenation (compiled to `StringBuilder` by javac) works on native strings the same as on literals.

Dispatch goes through `Class.hook` (`hook_for` in `runtime.rs`, set at class load). `invoke` and `call_native` call `hook_native` first; a hook returns `None` to fall through to the dex body or `UNRESOLVED`. There is no per-class `match` in `runtime.rs` any more.

| Hook | File | Content |
|---|---|---|
| `HOOK_STRING` | `strings.rs` | `length`, `charAt`, `hashCode`, `equals`, `equalsIgnoreCase`, `compareTo`, `indexOf`/`lastIndexOf` (char and String), `startsWith`, `endsWith`, `contains`, `substring`/`fastSubstring`, `trim`, case mapping, `replace`/`doReplace`, `concat`, `intern`, `isEmpty`, `toCharArray`, `getCharsNoCheck`, `getBytes`/`fillBytesLatin1`, `doRepeat` |
| `HOOK_FACTORY` | `strings.rs` | `StringFactory.new*`: UTF-8, ISO-8859-1, US-ASCII, UTF-16 (LE/BE/BOM) by charset name or `Charset` object, `newStringFromUtf16Bytes`; `new String(...)` call sites are routed to `StringFactory` |
| `HOOK_OBJECT`, `HOOK_CLASS`, `HOOK_SYSTEM`, `HOOK_MATH`, `HOOK_BITS`, `HOOK_UTF16`, `HOOK_CHAR` | `natives.rs` | `getClass`, `identityHashCodeNative`, `internalClone`, `Class.getName`/`getSimpleName`, `System.arraycopy`, properties and time, `Math`/`StrictMath`, float/double bit conversion, `StringUTF16.isBigEndian`, `Character` Impl methods |
| `HOOK_REFERENCE`, `HOOK_VMRUNTIME` | `natives.rs` | `Reference.getReferent`/`refersTo0`/`clearReferent`, `VMRuntime` queries |
| `HOOK_THREAD`, `HOOK_REGISTRY`, `HOOK_PATTERN`, `HOOK_MATCHER` | `rx.rs` | `Thread.currentThread`/`sleep`, `NativeAllocationRegistry`, ICU regex natives backed by `rx.rs` |

Verified with testapp3 (StringTest.apk, 267 checks: StringBuilder append/insert/delete/reverse/growth/unicode, `+` concatenation, `String.valueOf`, number conversion, case, compare, split, bytes and charsets, bounds exceptions, identity, collections, regex, floating point, `String.format`): `winart test testapp3/out/StringTest.apk --seen "STRINGTEST ALL PASS"`. WinCalc and LayoutTest still pass.

## Threads, Looper and Handler

Java threads are real Windows threads (`threads.rs`). One interpreter instance is shared behind a global interpreter lock: a thread runs Dalvik code only while it holds the GIL, and gives it up at `Thread.sleep`, `Object.wait`, contended monitors, every 1024 interpreted instructions when more than one thread exists, and while the Win32 loop waits in `GetMessageW`. Threads start in `Thread.nativeCreate`, run `Thread.run()` on their own native stack, report uncaught exceptions through `dispatchUncaughtException` and unblock `join()` on exit. `monitor-enter/exit`, `synchronized` methods and `wait/notify/notifyAll` are real once the first extra thread starts (single-threaded runs keep them as no-ops). `Thread.currentThread`, `isAlive`, `getState`, `interrupt`, `holdsLock` come from the libcore class through `HOOK_THREAD`. The GC runs from the UI thread at top level while workers are parked; it marks every worker's live interpreter frames (registered in `Threads.frames`) plus thread objects, and call depth is tracked per thread.

`android.os` in `framework/src` has `Looper`, `Handler`, `Message` (with pool), `MessageQueue` (time-ordered, thread-safe, `removeMessages`/`removeCallbacks`/`hasMessages`, `quit`/`quitSafely`) and `SystemClock`. Worker loopers block in `MessageQueue.next()` on a real monitor wait. The main looper is created by `Host.init` on the UI thread and is never run by `Looper.loop()`; `Native.wakeMain` posts a `0x8001` message to the window, and the window procedure (and `WM_TIMER` for delayed messages) calls `Host.pumpMain()`, which dispatches due messages on the UI thread and returns the delay to the next one (0 more now, -1 none). `winart test --wait MS` drives the same pump without a window. Views touched from another thread while attached to the activity throw `ViewRootImpl.CalledFromWrongThreadException`; `View.post/postDelayed/removeCallbacks`, `Activity.runOnUiThread` and `Context.getMainLooper` are provided.

Verified with testapp4 (ThreadTest.apk): main looper, `post`, `postDelayed`, `sendMessage`/`sendEmptyMessage`, `postAtFrontOfQueue`, callback order, `removeCallbacks`/`removeMessages`/`removeCallbacksAndMessages`, worker `Thread.start`, worker-to-main posts, worker `Looper.prepare/loop/quit/quitSafely`, wait/notify handshakes, `join`, wrong-thread UI rejection: `winart test testapp4/out/ThreadTest.apk --seen "THREADTEST ALL PASS" --wait 8000` (also with `--gc`). `tools/regress.ps1` runs WinCalc, LayoutTest, StringTest, ThreadTest and ThreadTest with `--gc`.
