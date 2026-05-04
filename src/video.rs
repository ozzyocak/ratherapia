use std::fs::{create_dir_all, File};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

use anyhow::{Context, Result};
use ratatui::backend::TestBackend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::style::{Color, Modifier};
use ratatui::Terminal;

use crate::app::{App, NoiseTexture, Waveform};
use crate::audio::AudioControl;
use crate::ui;

const TERMINAL_COLS: u16 = 88;
const TERMINAL_ROWS: u16 = 28;
const CELL_WIDTH: u32 = 10;
const CELL_HEIGHT: u32 = 16;
const FPS: u32 = 30;
const SECONDS: u32 = 6;

#[derive(Clone, Copy)]
struct Rgb(u8, u8, u8);

struct Image {
    width: u32,
    height: u32,
    pixels: Vec<Rgb>,
}

pub fn render_demo(output: String) -> Result<()> {
    let output = Path::new(&output);
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }

    let mut app = App::new(AudioControl::silent("video: rust avi"));
    let mut terminal = Terminal::new(TestBackend::new(TERMINAL_COLS, TERMINAL_ROWS))?;
    let mut frames = Vec::with_capacity((FPS * SECONDS) as usize);

    for frame_index in 0..(FPS * SECONDS) {
        apply_timeline(&mut app, frame_index);
        app.tick(1.0 / FPS as f32);

        terminal.draw(|frame| ui::draw(frame, &app))?;
        frames.push(rasterize(terminal.backend().buffer()));
    }

    write_uncompressed_avi(output, &frames, FPS)?;
    println!("rendered {} frames to {}", frames.len(), output.display());

    Ok(())
}

fn apply_timeline(app: &mut App, frame_index: u32) {
    match frame_index {
        0 => {
            app.set_waveform(Waveform::Sine);
            app.play_note(0);
        }
        34 => app.play_note(4),
        60 => {
            app.set_waveform(Waveform::Square);
            app.set_noise_texture(NoiseTexture::White);
            app.play_note(7);
        }
        92 => app.play_note(11),
        120 => {
            app.set_waveform(Waveform::Saw);
            app.set_noise_texture(NoiseTexture::Rain);
            app.adjust_cutoff(1_500.0);
            app.play_note(2);
        }
        150 => {
            app.set_waveform(Waveform::Triangle);
            app.set_noise_texture(NoiseTexture::Wind);
            app.adjust_gain(0.12);
            app.play_note(9);
        }
        _ => {}
    }

    app.gate_open = (frame_index / 15) % 4 != 3;
}

fn rasterize(buffer: &Buffer) -> Image {
    let width = buffer.area.width as u32 * CELL_WIDTH;
    let height = buffer.area.height as u32 * CELL_HEIGHT;
    let mut image = Image {
        width,
        height,
        pixels: vec![Rgb(8, 10, 14); (width * height) as usize],
    };

    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            draw_cell(&mut image, x as u32, y as u32, cell);
        }
    }

    image
}

fn draw_cell(image: &mut Image, cell_x: u32, cell_y: u32, cell: &Cell) {
    let x = cell_x * CELL_WIDTH;
    let y = cell_y * CELL_HEIGHT;
    let bg = rgb_for_bg(cell.bg);
    let mut fg = rgb_for_fg(cell.fg);

    if cell.modifier.contains(Modifier::BOLD) {
        fg = brighten(fg);
    }

    fill_rect(image, x, y, CELL_WIDTH, CELL_HEIGHT, bg);
    draw_symbol(image, x, y, cell.symbol(), fg);
}

fn draw_symbol(image: &mut Image, x: u32, y: u32, symbol: &str, color: Rgb) {
    match symbol {
        " " => {}
        "─" | "━" => fill_rect(image, x, y + CELL_HEIGHT / 2, CELL_WIDTH, 1, color),
        "│" | "┃" => fill_rect(image, x + CELL_WIDTH / 2, y, 1, CELL_HEIGHT, color),
        "┌" | "┐" | "└" | "┘" | "├" | "┤" | "┬" | "┴" | "┼" => {
            draw_box_symbol(image, x, y, symbol, color)
        }
        "█" | "▓" | "▒" | "░" => {
            fill_rect(image, x + 1, y + 1, CELL_WIDTH - 2, CELL_HEIGHT - 2, color)
        }
        _ => {
            let ch = symbol.chars().next().unwrap_or(' ');
            draw_glyph(image, x + 1, y + 1, ch, color);
        }
    }
}

fn draw_box_symbol(image: &mut Image, x: u32, y: u32, symbol: &str, color: Rgb) {
    let mid_x = x + CELL_WIDTH / 2;
    let mid_y = y + CELL_HEIGHT / 2;
    let left = matches!(symbol, "┐" | "┘" | "┤" | "┬" | "┴" | "┼");
    let right = matches!(symbol, "┌" | "└" | "├" | "┬" | "┴" | "┼");
    let up = matches!(symbol, "└" | "┘" | "├" | "┤" | "┴" | "┼");
    let down = matches!(symbol, "┌" | "┐" | "├" | "┤" | "┬" | "┼");

    if left {
        fill_rect(image, x, mid_y, CELL_WIDTH / 2 + 1, 1, color);
    }
    if right {
        fill_rect(image, mid_x, mid_y, CELL_WIDTH / 2, 1, color);
    }
    if up {
        fill_rect(image, mid_x, y, 1, CELL_HEIGHT / 2 + 1, color);
    }
    if down {
        fill_rect(image, mid_x, mid_y, 1, CELL_HEIGHT / 2, color);
    }
}

fn draw_glyph(image: &mut Image, x: u32, y: u32, ch: char, color: Rgb) {
    let pattern = glyph_pattern(ch);
    for (row, bits) in pattern.iter().enumerate() {
        for col in 0..5 {
            if (bits >> (4 - col)) & 1 == 1 {
                fill_rect(image, x + col * 2, y + row as u32 * 2, 2, 2, color);
            }
        }
    }
}

fn glyph_pattern(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'B' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
        'C' => [
            0b01111, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b01111,
        ],
        'D' => [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
        'E' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
        'F' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'G' => [
            0b01111, 0b10000, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
        'H' => [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
        'I' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b11111,
        ],
        'J' => [
            0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100,
        ],
        'K' => [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
        'L' => [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
        'M' => [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
        'N' => [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
        'O' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'P' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
        'Q' => [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
        'R' => [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
        'S' => [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        'T' => [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'U' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
        'V' => [
            0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b01010, 0b00100,
        ],
        'W' => [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b10101, 0b01010,
        ],
        'X' => [
            0b10001, 0b01010, 0b01010, 0b00100, 0b01010, 0b01010, 0b10001,
        ],
        'Y' => [
            0b10001, 0b01010, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
        'Z' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
        '0' => [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
        '1' => [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
        '2' => [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
        '3' => [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
        '4' => [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
        '5' => [
            0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
        ],
        '6' => [
            0b01111, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
        '7' => [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
        '8' => [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
        '9' => [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b11110,
        ],
        '.' => [0, 0, 0, 0, 0, 0b01100, 0b01100],
        ':' => [0, 0b01100, 0b01100, 0, 0b01100, 0b01100, 0],
        '-' => [0, 0, 0, 0b11111, 0, 0, 0],
        '+' => [0, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0],
        '/' => [
            0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000,
        ],
        '#' => [
            0b01010, 0b11111, 0b01010, 0b01010, 0b11111, 0b01010, 0b01010,
        ],
        '*' => [0, 0b10101, 0b01110, 0b11111, 0b01110, 0b10101, 0],
        '@' => [
            0b01110, 0b10001, 0b10111, 0b10101, 0b10111, 0b10000, 0b01110,
        ],
        '=' => [0, 0, 0b11111, 0, 0b11111, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 0b11111],
        _ => [0, 0, 0, 0, 0, 0, 0],
    }
}

fn fill_rect(image: &mut Image, x: u32, y: u32, width: u32, height: u32, color: Rgb) {
    for py in y..(y + height).min(image.height) {
        for px in x..(x + width).min(image.width) {
            image.pixels[(py * image.width + px) as usize] = color;
        }
    }
}

fn rgb_for_fg(color: Color) -> Rgb {
    match color {
        Color::Reset => Rgb(214, 222, 235),
        Color::Black => Rgb(8, 10, 14),
        Color::Red => Rgb(210, 74, 74),
        Color::Green => Rgb(70, 190, 116),
        Color::Yellow => Rgb(232, 190, 84),
        Color::Blue => Rgb(78, 132, 214),
        Color::Magenta => Rgb(204, 104, 214),
        Color::Cyan => Rgb(72, 205, 224),
        Color::Gray => Rgb(150, 158, 170),
        Color::DarkGray => Rgb(80, 88, 100),
        Color::LightRed => Rgb(255, 105, 105),
        Color::LightGreen => Rgb(108, 228, 146),
        Color::LightYellow => Rgb(255, 218, 112),
        Color::LightBlue => Rgb(118, 166, 255),
        Color::LightMagenta => Rgb(232, 134, 255),
        Color::LightCyan => Rgb(108, 232, 246),
        Color::White => Rgb(244, 248, 255),
        Color::Rgb(r, g, b) => Rgb(r, g, b),
        Color::Indexed(index) => indexed_color(index),
    }
}

fn rgb_for_bg(color: Color) -> Rgb {
    match color {
        Color::Reset => Rgb(8, 10, 14),
        _ => rgb_for_fg(color),
    }
}

fn indexed_color(index: u8) -> Rgb {
    const BASIC: [Rgb; 16] = [
        Rgb(8, 10, 14),
        Rgb(210, 74, 74),
        Rgb(70, 190, 116),
        Rgb(232, 190, 84),
        Rgb(78, 132, 214),
        Rgb(204, 104, 214),
        Rgb(72, 205, 224),
        Rgb(150, 158, 170),
        Rgb(80, 88, 100),
        Rgb(255, 105, 105),
        Rgb(108, 228, 146),
        Rgb(255, 218, 112),
        Rgb(118, 166, 255),
        Rgb(232, 134, 255),
        Rgb(108, 232, 246),
        Rgb(244, 248, 255),
    ];

    BASIC[index.min(15) as usize]
}

fn brighten(color: Rgb) -> Rgb {
    Rgb(
        color.0.saturating_add(24),
        color.1.saturating_add(24),
        color.2.saturating_add(24),
    )
}

fn write_uncompressed_avi(path: &Path, frames: &[Image], fps: u32) -> Result<()> {
    let first = frames.first().context("no frames to write")?;
    let width = first.width;
    let height = first.height;
    let row_stride = (width * 3).next_multiple_of(4);
    let frame_size = row_stride * height;
    let mut file = File::create(path).with_context(|| format!("creating {}", path.display()))?;

    file.write_all(b"RIFF")?;
    let riff_size_pos = file.stream_position()?;
    write_u32(&mut file, 0)?;
    file.write_all(b"AVI ")?;

    write_header_list(
        &mut file,
        width,
        height,
        fps,
        frames.len() as u32,
        frame_size,
    )?;

    file.write_all(b"LIST")?;
    let movi_size_pos = file.stream_position()?;
    write_u32(&mut file, 0)?;
    file.write_all(b"movi")?;
    let movi_data_start = file.stream_position()?;
    let mut index = Vec::with_capacity(frames.len());

    for frame in frames {
        let chunk_start = file.stream_position()?;
        file.write_all(b"00db")?;
        write_u32(&mut file, frame_size)?;
        write_dib_frame(&mut file, frame, row_stride)?;
        if frame_size % 2 == 1 {
            file.write_all(&[0])?;
        }
        index.push(((chunk_start - movi_data_start) as u32, frame_size));
    }

    let after_movi = file.stream_position()?;
    file.seek(SeekFrom::Start(movi_size_pos))?;
    write_u32(&mut file, (after_movi - movi_data_start + 4) as u32)?;
    file.seek(SeekFrom::Start(after_movi))?;

    file.write_all(b"idx1")?;
    write_u32(&mut file, (index.len() * 16) as u32)?;
    for (offset, size) in index {
        file.write_all(b"00db")?;
        write_u32(&mut file, 0x10)?;
        write_u32(&mut file, offset)?;
        write_u32(&mut file, size)?;
    }

    let file_len = file.stream_position()?;
    file.seek(SeekFrom::Start(riff_size_pos))?;
    write_u32(&mut file, (file_len - 8) as u32)?;

    Ok(())
}

fn write_header_list(
    file: &mut File,
    width: u32,
    height: u32,
    fps: u32,
    total_frames: u32,
    frame_size: u32,
) -> Result<()> {
    let hdrl_size = 4 + 8 + 56 + 4 + 8 + 56 + 8 + 40;
    file.write_all(b"LIST")?;
    write_u32(file, hdrl_size)?;
    file.write_all(b"hdrl")?;

    file.write_all(b"avih")?;
    write_u32(file, 56)?;
    write_u32(file, 1_000_000 / fps)?;
    write_u32(file, frame_size * fps)?;
    write_u32(file, 0)?;
    write_u32(file, 0x10)?;
    write_u32(file, total_frames)?;
    write_u32(file, 0)?;
    write_u32(file, 1)?;
    write_u32(file, frame_size)?;
    write_u32(file, width)?;
    write_u32(file, height)?;
    for _ in 0..4 {
        write_u32(file, 0)?;
    }

    file.write_all(b"LIST")?;
    write_u32(file, 4 + 8 + 56 + 8 + 40)?;
    file.write_all(b"strl")?;

    file.write_all(b"strh")?;
    write_u32(file, 56)?;
    file.write_all(b"vids")?;
    file.write_all(b"DIB ")?;
    write_u32(file, 0)?;
    write_u16(file, 0)?;
    write_u16(file, 0)?;
    write_u32(file, 0)?;
    write_u32(file, 1)?;
    write_u32(file, fps)?;
    write_u32(file, 0)?;
    write_u32(file, total_frames)?;
    write_u32(file, frame_size)?;
    write_u32(file, u32::MAX)?;
    write_u32(file, 0)?;
    write_u16(file, 0)?;
    write_u16(file, 0)?;
    write_u16(file, width as u16)?;
    write_u16(file, height as u16)?;

    file.write_all(b"strf")?;
    write_u32(file, 40)?;
    write_u32(file, 40)?;
    write_i32(file, width as i32)?;
    write_i32(file, height as i32)?;
    write_u16(file, 1)?;
    write_u16(file, 24)?;
    write_u32(file, 0)?;
    write_u32(file, frame_size)?;
    write_i32(file, 0)?;
    write_i32(file, 0)?;
    write_u32(file, 0)?;
    write_u32(file, 0)?;

    Ok(())
}

fn write_dib_frame(file: &mut File, image: &Image, row_stride: u32) -> Result<()> {
    let padding = (row_stride - image.width * 3) as usize;
    let mut row = vec![0; row_stride as usize];
    for y in (0..image.height).rev() {
        row.fill(0);
        for x in 0..image.width {
            let Rgb(r, g, b) = image.pixels[(y * image.width + x) as usize];
            let offset = (x * 3) as usize;
            row[offset] = b;
            row[offset + 1] = g;
            row[offset + 2] = r;
        }
        file.write_all(&row[..row.len() - padding])?;
        file.write_all(&row[row.len() - padding..])?;
    }
    Ok(())
}

fn write_u16(file: &mut File, value: u16) -> Result<()> {
    file.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn write_u32(file: &mut File, value: u32) -> Result<()> {
    file.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn write_i32(file: &mut File, value: i32) -> Result<()> {
    file.write_all(&value.to_le_bytes())?;
    Ok(())
}
