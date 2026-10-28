use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn find_rc_exe() -> Option<PathBuf> {
    if Command::new("rc.exe").arg("/?").output().is_ok() {
        return Some(PathBuf::from("rc.exe"));
    }

    let search_roots = [
        r"C:\Program Files (x86)\Windows Kits\10\bin",
        r"C:\Program Files\Windows Kits\10\bin",
    ];

    for root in search_roots {
        let root_path = Path::new(root);
        if let Ok(entries) = std::fs::read_dir(root_path) {
            let mut dirs: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.is_dir())
                .collect();
            dirs.sort();
            dirs.reverse();

            for dir in dirs {
                let candidate = dir.join("x64").join("rc.exe");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        println!("cargo:rerun-if-changed=app.rc");
        println!("cargo:rerun-if-changed=assets/icon.ico");

        let out_dir = env::var("OUT_DIR").expect("OUT_DIR must be present");
        let res_path = Path::new(&out_dir).join("app.res");

        if let Some(rc) = find_rc_exe() {
            let status = Command::new(&rc)
                .args(["/nologo", "/fo", res_path.to_str().unwrap(), "app.rc"])
                .status();

            if let Ok(s) = status {
                if s.success() {
                    println!("cargo:rustc-link-arg={}", res_path.display());
                }
            }
        }
    }
}
