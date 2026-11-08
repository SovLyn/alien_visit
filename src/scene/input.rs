//! Canvas-owned pointer gestures; keyboard handling is movement-only.
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{Event, EventTarget, HtmlCanvasElement, KeyboardEvent, PointerEvent, WheelEvent};

#[derive(Default)]
pub struct Input {
    pub look: [f64; 2],
    pub zoom: f64,
    pub keys: [bool; 5], // forward, back, left, right, shift
    pointers: Vec<(i32, f64, f64)>,
}
fn listen(target: &EventTarget, name: &str, f: impl FnMut(Event) + 'static) -> Result<(), JsValue> {
    let cb = Closure::<dyn FnMut(Event)>::new(f);
    target.add_event_listener_with_callback(name, cb.as_ref().unchecked_ref())?;
    cb.forget();
    Ok(())
}
pub fn install(canvas: &HtmlCanvasElement) -> Result<Rc<RefCell<Input>>, JsValue> {
    let input = Rc::new(RefCell::new(Input::default()));
    canvas.set_attribute("tabindex", "0")?;
    canvas.style().set_property("touch-action", "none")?;
    let i = input.clone();
    let c = canvas.clone();
    listen(canvas.as_ref(), "pointerdown", move |event| {
        let Some(e) = event.dyn_ref::<PointerEvent>() else {
            return;
        };
        if e.button() != 0 {
            return;
        }
        event.prevent_default();
        let _ = c.focus();
        let _ = c.set_pointer_capture(e.pointer_id());
        let mut s = i.borrow_mut();
        s.pointers.retain(|p| p.0 != e.pointer_id());
        s.pointers
            .push((e.pointer_id(), e.client_x() as f64, e.client_y() as f64));
    })?;
    let i = input.clone();
    let c = canvas.clone();
    listen(canvas.as_ref(), "pointermove", move |event| {
        let Some(e) = event.dyn_ref::<PointerEvent>() else {
            return;
        };
        let locked = c
            .owner_document()
            .and_then(|d| d.pointer_lock_element())
            .map(|p| p == c.clone().into())
            .unwrap_or(false);
        let mut s = i.borrow_mut();
        if locked {
            s.look[0] += e.movement_x() as f64;
            s.look[1] += e.movement_y() as f64;
            return;
        }
        let Some(index) = s.pointers.iter().position(|p| p.0 == e.pointer_id()) else {
            return;
        };
        event.prevent_default();
        let old = s.pointers[index];
        let x = e.client_x() as f64;
        let y = e.client_y() as f64;
        if s.pointers.len() == 2 {
            let other = s.pointers[1 - index];
            let before = (old.1 - other.1).hypot(old.2 - other.2);
            let after = (x - other.1).hypot(y - other.2);
            if before > 2. && after > 2. {
                s.zoom += (before / after).ln();
            }
        } else {
            s.look[0] += x - old.1;
            s.look[1] += y - old.2;
        }
        s.pointers[index] = (old.0, x, y);
    })?;
    for name in ["pointerup", "pointercancel", "lostpointercapture"] {
        let i = input.clone();
        listen(canvas.as_ref(), name, move |event| {
            if let Some(e) = event.dyn_ref::<PointerEvent>() {
                i.borrow_mut().pointers.retain(|p| p.0 != e.pointer_id());
            }
        })?;
    }
    let c = canvas.clone();
    listen(canvas.as_ref(), "dblclick", move |e| {
        e.prevent_default();
        c.request_pointer_lock();
    })?;
    let i = input.clone();
    listen(canvas.as_ref(), "wheel", move |event| {
        if let Some(e) = event.dyn_ref::<WheelEvent>() {
            event.prevent_default();
            let unit = if e.delta_mode() == 1 {
                16.
            } else if e.delta_mode() == 2 {
                500.
            } else {
                1.
            };
            i.borrow_mut().zoom += (e.delta_y() * unit * 0.0012).clamp(-1., 1.);
        }
    })?;
    if let Some(doc) = canvas.owner_document() {
        for down in [true, false] {
            let i = input.clone();
            listen(
                doc.as_ref(),
                if down { "keydown" } else { "keyup" },
                move |event| {
                    let Some(e) = event.dyn_ref::<KeyboardEvent>() else {
                        return;
                    };
                    let editable = e
                        .target()
                        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                        .map(|el| {
                            matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
                                || el.get_attribute("contenteditable").as_deref() == Some("true")
                        })
                        .unwrap_or(false);
                    if down && (editable || e.ctrl_key() || e.meta_key() || e.alt_key()) {
                        return;
                    }
                    let key = match e.code().as_str() {
                        "KeyW" | "ArrowUp" => Some(0),
                        "KeyS" | "ArrowDown" => Some(1),
                        "KeyA" | "ArrowLeft" => Some(2),
                        "KeyD" | "ArrowRight" => Some(3),
                        "ShiftLeft" | "ShiftRight" => Some(4),
                        _ => None,
                    };
                    if let Some(k) = key {
                        i.borrow_mut().keys[k] = down;
                        if k != 4 {
                            event.prevent_default();
                        }
                    }
                },
            )?;
        }
    }
    if let Some(window) = web_sys::window() {
        let i = input.clone();
        listen(window.as_ref(), "blur", move |_| {
            let mut s = i.borrow_mut();
            s.keys = [false; 5];
            s.pointers.clear();
            s.look = [0.; 2];
        })?;
    }
    Ok(input)
}
