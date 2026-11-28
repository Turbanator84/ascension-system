//! Ascension System game engine.
//!
//! The same code runs in two places:
//! - natively inside the desktop app (reminders, background Brightspace sync), and
//! - as WebAssembly inside the app's window, where the UI calls it through a
//!   tiny JSON interface (`alloc` / `call` / `dealloc`, see [`wasm`]).

pub mod date;
pub mod engine;
pub mod ics;
pub mod js;
pub mod nutrition;

use serde_json::{json, Value};

/// Run one engine request. Requests and replies are JSON so the UI can pass
/// its saved state straight through:
///
/// - `{"op":"compute","state":…,"today":"YYYY-MM-DD"}` → derived totals
/// - `{"op":"targets","state":…}` → calorie and macro targets
/// - `{"op":"import_ics","state":…,"ics":"…","today":…,"now":<ms>}` → `{state, msg, added, updated, reviews}`
/// - `{"op":"version"}`
///
/// Replies are `{"ok": …}` or `{"error": "…"}`.
pub fn handle(req: &Value, tz_offset_min: &dyn Fn(f64) -> i64) -> Value {
    let op = req.get("op").and_then(Value::as_str).unwrap_or("");
    let today = req.get("today").and_then(Value::as_str).unwrap_or("");
    let state = req.get("state").cloned().unwrap_or(Value::Null);
    match op {
        "version" => json!({"ok": env!("CARGO_PKG_VERSION")}),
        "compute" => json!({"ok": engine::compute(&state, today)}),
        "targets" => {
            let t = nutrition::targets(&state);
            json!({"ok": {"bmr": t.bmr, "tdee": t.tdee, "delta": t.delta, "kcal": t.kcal, "p": t.p, "c": t.c, "f": t.f}})
        }
        "import_ics" => {
            let mut state = state;
            let ics = req.get("ics").and_then(Value::as_str).unwrap_or("");
            let now = req.get("now").and_then(Value::as_f64).unwrap_or(0.0);
            let mut ids = ics::Ids::new(now.to_bits());
            match ics::import_ics(&mut state, ics, today, &date::iso_from_ms(now), &mut ids, tz_offset_min) {
                None => json!({"ok": {"ok": false, "msg": "No calendar events found. Check that you used the Brightspace calendar feed."}}),
                Some(s) => json!({"ok": {
                    "ok": true, "state": state, "added": s.added, "updated": s.updated, "reviews": s.reviews,
                    "msg": format!("{} new, {} updated, {} review sessions scheduled.", s.added, s.updated, s.reviews)
                }}),
            }
        }
        _ => json!({"error": format!("unknown op {op:?}")}),
    }
}

/// The WebAssembly interface. JavaScript writes a JSON request into memory it
/// got from `alloc`, calls `call(ptr, len)`, and reads back a reply laid out as
/// a 4-byte little-endian length followed by UTF-8 JSON, then frees both.
#[cfg(target_arch = "wasm32")]
pub mod wasm {
    use std::alloc::{alloc as raw_alloc, dealloc as raw_dealloc, Layout};

    #[link(wasm_import_module = "env")]
    extern "C" {
        /// The user's UTC offset in minutes at an instant (`-new Date(ms).getTimezoneOffset()`).
        fn tz_offset_min(epoch_ms: f64) -> i32;
    }

    fn layout(len: usize) -> Layout {
        Layout::from_size_align(len.max(1), 8).expect("valid layout")
    }

    #[no_mangle]
    pub extern "C" fn alloc(len: usize) -> *mut u8 {
        // SAFETY: the layout has non-zero size.
        unsafe { raw_alloc(layout(len)) }
    }

    /// # Safety
    /// `ptr` must come from `alloc(len)` (or a `call` reply of `len` bytes) and be freed once.
    #[no_mangle]
    pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
        raw_dealloc(ptr, layout(len));
    }

    /// # Safety
    /// `ptr..ptr+len` must be a live allocation from `alloc` holding UTF-8 JSON.
    #[no_mangle]
    pub unsafe extern "C" fn call(ptr: *const u8, len: usize) -> *mut u8 {
        let input = std::slice::from_raw_parts(ptr, len);
        let reply = match serde_json::from_slice::<serde_json::Value>(input) {
            Ok(req) => super::handle(&req, &|ms| i64::from(tz_offset_min(ms))),
            Err(e) => serde_json::json!({"error": format!("bad request: {e}")}),
        };
        let body = serde_json::to_vec(&reply).unwrap_or_else(|_| b"{\"error\":\"encode\"}".to_vec());
        let out = alloc(body.len() + 4);
        let n = u32::try_from(body.len()).unwrap_or(u32::MAX);
        std::ptr::copy_nonoverlapping(n.to_le_bytes().as_ptr(), out, 4);
        std::ptr::copy_nonoverlapping(body.as_ptr(), out.add(4), body.len());
        out
    }
}
