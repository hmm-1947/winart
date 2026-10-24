mod axml;
mod dex;
mod runtime;
mod ui;
mod zip;

use std::collections::BTreeSet;

fn dex_names(z: &zip::Zip) -> Vec<String> {
    let mut v = vec!["classes.dex".to_string()];
    let mut n = 2;
    loop {
        let name = format!("classes{}.dex", n);
        if z.entries.iter().any(|e| e.name == name) {
            v.push(name);
            n += 1;
        } else {
            break;
        }
    }
    v.retain(|n| z.entries.iter().any(|e| &e.name == n));
    v
}

fn info(path: &str) -> Result<(), String> {
    let z = zip::Zip::open(path)?;
    println!("entries:");
    for e in &z.entries {
        println!("  {} ({} bytes)", e.name, e.size);
    }
    if let Some(m) = z.read("AndroidManifest.xml") {
        let mf = axml::parse_manifest(&m)?;
        println!("package: {}", mf.package);
        println!("launcher: {}", mf.launcher.clone().unwrap_or_default());
        println!("minSdk: {:?} targetSdk: {:?}", mf.min_sdk, mf.target_sdk);
        println!("permissions: {:?}", mf.permissions);
    }
    for name in dex_names(&z) {
        let d = dex::Dex::parse(z.read(&name).ok_or("dex read failed")?)?;
        let defined: BTreeSet<String> = d.classes.iter().map(|c| d.type_desc(c.class).to_string()).collect();
        println!("{}: {} classes, {} methods", name, d.classes.len(), d.methods.len());
        let mut ext_types = BTreeSet::new();
        for i in 0..d.types.len() {
            let t = d.type_desc(i as u32);
            if t.starts_with('L') && !defined.contains(t) {
                ext_types.insert(t.to_string());
            }
        }
        println!("external classes ({}):", ext_types.len());
        for t in &ext_types {
            println!("  {}", t);
        }
        let mut ext_methods = BTreeSet::new();
        for (i, m) in d.methods.iter().enumerate() {
            let c = d.type_desc(m.class as u32);
            if !defined.contains(c) && c.starts_with('L') {
                ext_methods.insert(format!("{}->{}{}", c, d.method_name(i as u32), d.sig(m.proto)));
            }
        }
        println!("external methods ({}):", ext_methods.len());
        for t in &ext_methods {
            println!("  {}", t);
        }
    }
    Ok(())
}

fn boot_path() -> Option<std::path::PathBuf> {
    if std::env::var("WINART_NOBOOT").is_ok() {
        return None;
    }
    if let Ok(p) = std::env::var("WINART_BOOT") {
        return Some(p.into());
    }
    let mut d = std::env::current_exe().ok()?;
    while d.pop() {
        let c = d.join("boot").join("core.dex");
        if c.exists() {
            return Some(c);
        }
    }
    None
}

fn add_boot(rt: &mut runtime::Runtime) -> Result<(), String> {
    if let Some(p) = boot_path() {
        let data = std::fs::read(&p).map_err(|e| format!("boot read {}: {}", p.display(), e))?;
        rt.add_dex(dex::Dex::parse(data)?);
    }
    Ok(())
}

fn press(path: &str, keys: &[String]) -> Result<(), String> {
    let z = zip::Zip::open(path)?;
    let mut rt = runtime::Runtime::new();
    add_boot(&mut rt)?;
    rt.start();
    for name in dex_names(&z) {
        rt.add_dex(dex::Dex::parse(z.read(&name).ok_or("dex read failed")?)?);
    }
    let obj = rt.new_instance("Lcom/winrt/calc/Calc;").map_err(|e| format!("new failed: {}", rt.class_name_of(e)))?;
    for k in keys {
        let s = rt.str_ref(k);
        match rt.call_virtual(obj, "press(Ljava/lang/String;)Ljava/lang/String;", vec![s]) {
            Ok(r) => println!("press {} -> {}", k, rt.string_value(r as u32)),
            Err(e) => return Err(format!("exception {}", rt.class_name_of(e))),
        }
    }
    Ok(())
}

fn load_app(path: &str) -> Result<(runtime::Runtime, axml::Manifest, u32), String> {
    let z = zip::Zip::open(path)?;
    let mf = axml::parse_manifest(&z.read("AndroidManifest.xml").ok_or("no manifest")?)?;
    let launcher = mf.launcher.clone().ok_or("no launcher activity")?;
    let mut rt = runtime::Runtime::new();
    add_boot(&mut rt)?;
    rt.start();
    let fw = boot_path().and_then(|p| p.parent().map(|d| d.join("framework.dex"))).ok_or("no framework.dex")?;
    rt.add_dex(dex::Dex::parse(std::fs::read(&fw).map_err(|e| format!("{}: {}", fw.display(), e))?)?);
    for name in dex_names(&z) {
        rt.add_dex(dex::Dex::parse(z.read(&name).ok_or("dex read failed")?)?);
    }
    let desc = format!("L{};", launcher.replace('.', "/"));
    let act = rt.new_instance(&desc).map_err(|e| format!("new activity failed: {}", rt.class_name_of(e)))?;
    Ok((rt, mf, act))
}

fn start_activity(rt: &mut runtime::Runtime, mf: &axml::Manifest, act: u32, density: f32) -> Result<(), String> {
    rt.add_root(act);
    rt.call_static("Landroid/winrt/Host;", "init(F)V", vec![density.to_bits()])
        .map_err(|e| format!("Host.init exception {}", rt.class_name_of(e)))?;
    let pkg = rt.str_ref(&mf.package);
    rt.call_static("Landroid/winrt/Host;", "start(Landroid/app/Activity;Ljava/lang/String;)V", vec![act, pkg])
        .map_err(|e| format!("Host.start exception {}", rt.class_name_of(e)))?;
    Ok(())
}

fn run_apk(path: &str) -> Result<(), String> {
    let (mut rt, mf, act) = load_app(path)?;
    let density = ui::create(&mut rt, &mf.package, 360, 640);
    start_activity(&mut rt, &mf, act, density)?;
    ui::pump();
    Ok(())
}

fn test_apk(path: &str, args: &[String]) -> Result<(), String> {
    let mut expect: Option<String> = None;
    let mut repeat = 1usize;
    let mut force_gc = false;
    let mut keys = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--expect" && i + 1 < args.len() {
            expect = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--repeat" && i + 1 < args.len() {
            repeat = args[i + 1].parse().map_err(|_| "bad --repeat".to_string())?;
            i += 2;
        } else if args[i] == "--gc" {
            force_gc = true;
            i += 1;
        } else {
            keys.push(args[i].clone());
            i += 1;
        }
    }
    let (mut rt, mf, act) = load_app(path)?;
    start_activity(&mut rt, &mf, act, 2.0)?;
    let heap0 = rt.live_objects();
    let mut peak = heap0;
    for round in 0..repeat {
        for k in &keys {
            let ks = rt.str_ref(k);
            let r = rt
                .call_static("Landroid/winrt/Host;", "clickByText(Ljava/lang/String;)Z", vec![ks])
                .map_err(|e| format!("click {} exception {}", k, rt.class_name_of(e)))?;
            if r & 1 == 0 {
                return Err(format!("no button with text {:?}", k));
            }
            if repeat == 1 {
                println!("click {} -> display {}", k, rt.last_text);
            }
            if force_gc {
                rt.gc();
            } else {
                rt.maybe_gc();
            }
            peak = peak.max(rt.live_objects());
        }
        if let Some(e) = &expect {
            if rt.last_text != *e {
                return Err(format!("FAIL round {} expected display {:?} got {:?}", round, e, rt.last_text));
            }
        }
    }
    println!("heap objects: start {} end {} peak {} gc runs {}", heap0, rt.live_objects(), peak, rt.gc_runs);
    if let Some(e) = expect {
        println!("PASS display == {:?}", e);
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let h = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(move || {
            let res = match args.get(1).map(|s| s.as_str()) {
                Some("info") if args.len() >= 3 => info(&args[2]),
                Some("press") if args.len() >= 3 => press(&args[2], &args[3..]),
                Some("run") if args.len() >= 3 => run_apk(&args[2]),
                Some("test") if args.len() >= 3 => test_apk(&args[2], &args[3..]),
                _ => Err("usage: winart info <apk> | press <apk> <keys...> | run <apk> | test <apk> [--expect S] <keys...>".to_string()),
            };
            if let Err(e) = res {
                eprintln!("error: {}", e);
                std::process::exit(1);
            }
        })
        .unwrap();
    h.join().unwrap();
}
