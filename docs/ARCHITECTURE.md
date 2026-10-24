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
