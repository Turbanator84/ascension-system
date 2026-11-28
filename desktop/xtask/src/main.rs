//! `cargo run -p xtask -- frontend`
//!
//! 1. Builds the game engine (`ascension-core`) for WebAssembly.
//! 2. Wraps the shared UI (`../src/app.html`, the same file the website uses)
//!    into `app/dist/index.html`, with the engine and the desktop bridge loaded first.
//!
//! Tauri runs this automatically before `cargo tauri dev` / `cargo tauri build`.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{exit, Command};

const RESET: &str = ":root{color-scheme:dark;padding-top:env(safe-area-inset-top,0px);padding-bottom:env(safe-area-inset-bottom,0px)}body{margin:0;font:14px system-ui,sans-serif}img{max-width:100%}[hidden]{display:none!important}";

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

fn fail(msg: &str) -> ! {
    eprintln!("xtask: {msg}");
    exit(1)
}

fn read(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_else(|e| fail(&format!("can't read {}: {e}", p.display())))
}

fn frontend(root: &Path) {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(&cargo)
        .current_dir(root)
        .args(["build", "-q", "-p", "ascension-core", "--release", "--target", "wasm32-unknown-unknown"])
        .status()
        .unwrap_or_else(|e| fail(&format!("can't run cargo: {e}")));
    if !status.success() {
        fail("building the WebAssembly engine failed. Install the target with: rustup target add wasm32-unknown-unknown");
    }
    let target_dir = env::var_os("CARGO_TARGET_DIR").map_or_else(|| root.join("target"), PathBuf::from);
    let wasm_path = target_dir.join("wasm32-unknown-unknown/release/ascension_core.wasm");
    let wasm = fs::read(&wasm_path).unwrap_or_else(|e| fail(&format!("can't read {}: {e}", wasm_path.display())));

    let web = root.join("app/web");
    let engine = read(&web.join("engine.js"));
    if !engine.contains("__ENGINE_WASM_BASE64__") {
        fail("engine.js is missing its placeholder");
    }
    let engine = engine.replace("__ENGINE_WASM_BASE64__", &base64(&wasm));
    let bridge = read(&web.join("desktop.js"));

    let app = read(&root.join("../src/app.html"));
    if !app.contains("window.AscensionBoot") {
        fail("src/app.html is missing the desktop boot hook (window.AscensionBoot)");
    }
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1,viewport-fit=cover\"><style>{RESET}</style><script src=\"engine.js\"></script><script src=\"desktop.js\"></script></head><body>\n{app}\n</body></html>\n"
    );

    let dist = root.join("app/dist");
    fs::create_dir_all(&dist).unwrap_or_else(|e| fail(&e.to_string()));
    for (name, body) in [("index.html", html.as_bytes()), ("engine.js", engine.as_bytes()), ("desktop.js", bridge.as_bytes())] {
        fs::write(dist.join(name), body).unwrap_or_else(|e| fail(&format!("can't write {name}: {e}")));
    }
    for icon in ["icon.png", "32x32.png"] {
        let _ = fs::copy(root.join("app/icons").join(icon), dist.join(icon));
    }
    println!("xtask: frontend ready in {} (engine {} KB)", dist.display(), wasm.len() / 1024);
}

fn main() {
    // desktop/ is the workspace root (xtask lives in desktop/xtask).
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("workspace root").to_path_buf();
    match env::args().nth(1).as_deref() {
        Some("frontend") => frontend(&root),
        _ => fail("usage: cargo run -p xtask -- frontend"),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_rfc4648() {
        assert_eq!(super::base64(b""), "");
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foobar"), "Zm9vYmFy");
    }
}
