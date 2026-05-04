use std::io::{self, Write};

use ratatui::layout::Rect;

const MOUSE_ID: u32 = 42;
const MOUSE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/objects/SpinyMouse.glb");
const MOUSE_FORMAT: &str = "glb";

pub fn enabled() -> bool {
    std::env::var_os("RATHERAPIA_RATTY").is_some()
}

pub fn register_mouse<W: Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(register_sequence().as_bytes())?;
    writer.flush()
}

pub fn clear_mouse<W: Write>(writer: &mut W) -> io::Result<()> {
    writer.write_all(delete_sequence().as_bytes())?;
    writer.flush()
}

pub fn mouse_place_sequence(
    area: Rect,
    color: [u8; 3],
    brightness: f32,
    pulse: f32,
    animate: bool,
) -> String {
    let row = area.y.saturating_add(area.height.saturating_sub(1) / 2);
    let col = area.x.saturating_add(area.width.saturating_sub(1) / 2);
    let scale = 1.0 + pulse.clamp(0.0, 1.0) * 0.16;
    let depth = 1.9 + pulse.clamp(0.0, 1.0) * 0.8;
    let animate = u8::from(animate);

    format!(
        "\x1b_ratty;g;p;id={MOUSE_ID};row={row};col={col};w={};h={};animate={animate};scale={scale:.2};depth={depth:.2};color={:02x}{:02x}{:02x};brightness={brightness:.2}\x1b\\",
        area.width.max(1),
        area.height.max(1),
        color[0],
        color[1],
        color[2],
    )
}

pub fn mouse_delete_sequence() -> String {
    delete_sequence()
}

fn register_sequence() -> String {
    format!("\x1b_ratty;g;r;id={MOUSE_ID};fmt={MOUSE_FORMAT};path={MOUSE_PATH}\x1b\\")
}

fn delete_sequence() -> String {
    format!("\x1b_ratty;g;d;id={MOUSE_ID}\x1b\\")
}
