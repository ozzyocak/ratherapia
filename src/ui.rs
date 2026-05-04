use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, NoiseTexture, VisualMode, Waveform};
use crate::ratty;

const KEYS: [(&str, &str); 12] = [
    ("z", "C"),
    ("s", "C#"),
    ("x", "D"),
    ("d", "D#"),
    ("c", "E"),
    ("v", "F"),
    ("g", "F#"),
    ("b", "G"),
    ("h", "G#"),
    ("n", "A"),
    ("j", "A#"),
    ("m", "B"),
];

pub fn draw(frame: &mut Frame, app: &App) {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(7),
        ])
        .split(frame.area());

    draw_header(frame, root[0], app);
    draw_engine(frame, root[1], app);
    draw_footer(frame, root[2], app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let title = Line::from(vec![
        Span::styled(
            "RATHERAPIA",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("ratatui synth engine", Style::default().fg(Color::DarkGray)),
    ]);

    let mode = Line::from(vec![
        label("visual "),
        Span::styled(app.visual_mode.as_str(), mode_style(app)),
        muted("  "),
        label("weather "),
        Span::styled(app.noise_texture.as_str(), texture_style(app)),
        muted("  "),
        Span::styled(&app.audio_status, Style::default().fg(Color::DarkGray)),
    ]);

    frame.render_widget(
        Paragraph::new(vec![title, mode])
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM)),
        area,
    );
}

fn draw_engine(frame: &mut Frame, area: Rect, app: &App) {
    let inner = area;

    let width = inner.width.max(1) as usize;
    let height = inner.height.max(1) as usize;
    let mut cells = vec![vec![' '; width]; height];
    let mut colors = vec![vec![Color::DarkGray; width]; height];

    let center_x = width / 2;
    let center_y = height / 2;
    draw_rain_backdrop(&mut cells, &mut colors, app);
    draw_fine_particles(&mut cells, &mut colors, app);
    draw_reactive_fine_particles(&mut cells, &mut colors, app);
    draw_rain_sheet(&mut cells, &mut colors, app, false);
    draw_floor_splashes(&mut cells, &mut colors, app, center_x, center_y);

    if app.visual_mode == VisualMode::TwoD {
        draw_center(&mut cells, &mut colors, center_x, center_y, app);
    } else {
        draw_3d_anchor_shadow(&mut cells, &mut colors, center_x, center_y, app);
    }
    draw_center_weather(&mut cells, &mut colors, center_x, center_y, app);
    draw_rain_sheet(&mut cells, &mut colors, app, true);
    draw_contact_rain(&mut cells, &mut colors, center_x, center_y, app);

    for row in 0..height {
        let line = cells[row]
            .iter()
            .enumerate()
            .map(|(col, ch)| Span::styled(ch.to_string(), Style::default().fg(colors[row][col])))
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(Line::from(line)), row_rect(inner, row));
    }

    draw_ratty_mouse(frame, inner, app);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let scope = scope_lines(app);
    let keyboard = keyboard_lines(app);

    frame.render_widget(
        Paragraph::new(vec![
            footer_status_line(app),
            Line::from(vec![label("scope "), scope[1].spans[0].clone()]),
            keyboard[0].clone(),
            keyboard[1].clone(),
            Line::from(vec![
                muted("space gate  "),
                label("0"),
                muted(" 2d/3d  "),
                label("1-4"),
                muted(" wave  "),
                label("5"),
                muted(" off  "),
                label("6"),
                muted(" white  "),
                label("7"),
                muted(" brown  "),
                label("8"),
                muted(" rain  "),
                label("9"),
                muted(" wind"),
            ]),
            Line::from(vec![
                label("[/]"),
                muted(" noise  "),
                label("arrows"),
                muted(" pitch/cutoff  "),
                label("+/-"),
                muted(" gain  "),
                label("z-m"),
                muted(" notes  "),
                label("q"),
                muted(" quit"),
            ]),
        ])
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .alignment(Alignment::Center),
        area,
    );
}

fn draw_rain_backdrop(cells: &mut [Vec<char>], colors: &mut [Vec<Color>], app: &App) {
    let height = cells.len();
    let width = cells[0].len();
    let frame = (app.visual_phase * 4.5) as i32;

    for y in 0..height {
        for x in 0..width {
            let grain = hash_cell(x as i32, y as i32, frame);
            if grain % 100 < 7 {
                cells[y][x] = if grain % 4 == 0 { '.' } else { ' ' };
                colors[y][x] = if grain % 11 == 0 {
                    Color::Gray
                } else {
                    Color::DarkGray
                };
            } else if grain % 240 == 0 {
                cells[y][x] = ':';
                colors[y][x] = Color::DarkGray;
            }
        }
    }

    let band_count = 4 + (rain_force(app) * 5.0) as usize;
    for band in 0..band_count {
        let x = ((width as f32 * (band as f32 + 0.5) / band_count as f32)
            + (app.visual_phase * 0.35 + band as f32).sin() * 2.0)
            .round()
            .clamp(0.0, width.saturating_sub(1) as f32) as usize;
        for y in 0..height {
            if (y + band) % 3 != 0 {
                cells[y][x] = '\'';
                colors[y][x] = Color::DarkGray;
            }
        }
    }
}

fn draw_fine_particles(cells: &mut [Vec<char>], colors: &mut [Vec<Color>], app: &App) {
    let height = cells.len();
    let width = cells[0].len();
    let frame = (app.visual_phase * 14.0) as i32;
    let density = 3 + (rain_force(app) * 6.0) as i32;

    for y in 0..height {
        for x in 0..width {
            let spark = hash_cell(x as i32, y as i32, frame + 29);
            if ((spark % 180) as i32) < density {
                cells[y][x] = match spark % 5 {
                    0 => '.',
                    1 => ',',
                    2 => '\'',
                    _ => '.',
                };
                colors[y][x] = if spark % 13 == 0 {
                    Color::Cyan
                } else {
                    Color::Gray
                };
            }
        }
    }
}

fn draw_reactive_fine_particles(cells: &mut [Vec<char>], colors: &mut [Vec<Color>], app: &App) {
    let height = cells.len();
    let width = cells[0].len();

    for particle in &app.particles {
        let x = (particle.x.clamp(0.0, 1.0) * (width.saturating_sub(1) as f32)).round() as usize;
        let y = (particle.y.clamp(0.0, 1.0) * (height.saturating_sub(1) as f32)).round() as usize;
        cells[y][x] = if particle.alpha > 0.7 { '*' } else { '.' };
        colors[y][x] = if particle.alpha > 0.7 {
            Color::White
        } else if particle.color_index % 2 == 0 {
            Color::Cyan
        } else {
            Color::Gray
        };
    }
}

fn draw_rain_sheet(
    cells: &mut [Vec<char>],
    colors: &mut [Vec<Color>],
    app: &App,
    foreground: bool,
) {
    let height = cells.len() as i32;
    let width = cells[0].len() as i32;
    let force = rain_force(app);
    let fall = (app.visual_phase * if foreground { 18.0 } else { 11.0 }) as i32;
    let density = ((if foreground { 7.0 } else { 30.0 }) * force) as i32;
    let slant = rain_slant(app);
    let max_tail = if foreground { 3 } else { 5 };

    for x in 0..width {
        let lane = hash_cell(x, if foreground { 43 } else { 17 }, 11);
        if ((lane % 100) as i32) >= density {
            continue;
        }

        let base_y = (lane as i32 + fall + x / 3).rem_euclid(height.max(1));
        let tail_len = 2 + (lane % max_tail) as i32;
        for tail in 0..tail_len {
            let y = base_y - tail;
            let x = x - (slant * tail) / 2;
            let ch = if foreground {
                if tail == 0 && lane % 11 == 0 {
                    '!'
                } else {
                    '|'
                }
            } else if tail < 2 {
                '|'
            } else {
                '\''
            };
            let color = if foreground && tail == 0 && lane % 11 == 0 {
                Color::White
            } else if foreground {
                Color::Cyan
            } else if tail < 2 {
                Color::Gray
            } else {
                Color::DarkGray
            };
            plot(cells, colors, x, y, ch, color);
        }
    }
}

fn draw_floor_splashes(
    cells: &mut [Vec<char>],
    colors: &mut [Vec<Color>],
    app: &App,
    center_x: usize,
    center_y: usize,
) {
    let width = cells[0].len() as i32;
    let floor_y = (center_y as i32 + 8).min(cells.len() as i32 - 2);
    let frame = (app.visual_phase * 8.0) as i32;

    for x in (0..width).step_by(8) {
        let seed = hash_cell(x, floor_y, frame);
        if seed % 100 < 24 {
            let y = floor_y + (seed % 3) as i32 - 1;
            plot(
                cells,
                colors,
                x,
                y,
                if seed % 2 == 0 { '_' } else { 'o' },
                Color::DarkGray,
            );
        }
    }

    let cx = center_x as i32;
    for dx in -17_i32..=17 {
        let seed = hash_cell(cx + dx, floor_y, frame + 7);
        if seed % 100 < 32 {
            plot(
                cells,
                colors,
                cx + dx,
                floor_y - (seed % 2) as i32,
                if dx.abs() < 6 { '*' } else { '.' },
                if dx.abs() < 6 {
                    Color::White
                } else {
                    Color::Gray
                },
            );
        }
    }
}

fn draw_center_weather(
    cells: &mut [Vec<char>],
    colors: &mut [Vec<Color>],
    center_x: usize,
    center_y: usize,
    app: &App,
) {
    draw_mouse_rain_hits(cells, colors, center_x, center_y, app);
    if app.noise_texture == NoiseTexture::Wind {
        draw_mouse_wind_hits(cells, colors, center_x, center_y, app);
    }
}

fn draw_mouse_rain_hits(
    cells: &mut [Vec<char>],
    colors: &mut [Vec<Color>],
    center_x: usize,
    center_y: usize,
    app: &App,
) {
    let cx = center_x as i32;
    let cy = center_y as i32;
    let fall = (app.visual_phase * 16.0) as i32;
    let density = 18 + (rain_force(app) * 18.0) as i32;
    let slant = rain_slant(app);

    for dx in -16_i32..=16 {
        let lane = hash_cell(cx + dx, 91, 7);
        if ((lane % 100) as i32) >= density {
            continue;
        }
        let base_y = cy - 9 + (lane as i32 + fall).rem_euclid(18);
        for tail in 0..4 {
            let x = cx + dx - (slant * tail) / 2;
            let y = base_y - tail;
            let ch = if tail == 0 && lane % 13 == 0 {
                '!'
            } else {
                '|'
            };
            let color = if tail == 0 && lane % 13 == 0 {
                Color::White
            } else {
                Color::Gray
            };
            plot(cells, colors, x, y, ch, color);
        }
    }

    for dx in -15_i32..=15 {
        let phase = ((app.visual_phase * 8.0) as i32 + dx.abs() * 3) % 13;
        if phase < 4 {
            plot(
                cells,
                colors,
                cx + dx,
                cy + 5 + (dx.abs() % 3 == 0) as i32,
                if phase < 2 { 'o' } else { '_' },
                Color::Gray,
            );
            if dx.abs() < 8 {
                plot(cells, colors, cx + dx / 2, cy - 6, '*', Color::White);
                plot(cells, colors, cx + dx / 2, cy + 3, ',', Color::White);
            }
        }
    }
}

fn draw_contact_rain(
    cells: &mut [Vec<char>],
    colors: &mut [Vec<Color>],
    center_x: usize,
    center_y: usize,
    app: &App,
) {
    let cx = center_x as i32;
    let cy = center_y as i32;
    let fall = (app.visual_phase * 10.0) as i32;

    for dx in -13_i32..=13 {
        let lane = hash_cell(cx + dx, 121, 19);
        if lane % 100 > 68 {
            continue;
        }

        let top = cy - 7 + (lane as i32 + fall + dx.abs()).rem_euclid(5);
        let contact = mouse_contact_y(dx, cy);
        if top <= contact {
            plot(cells, colors, cx + dx, contact - 1, '|', Color::Gray);
        }

        let impact = if lane % 9 == 0 {
            '*'
        } else if lane % 3 == 0 {
            '.'
        } else {
            '\''
        };
        let impact_color = if lane % 9 == 0 {
            Color::White
        } else {
            Color::Gray
        };
        plot(cells, colors, cx + dx, contact, impact, impact_color);

        if lane % 7 == 0 {
            plot(cells, colors, cx + dx - 1, contact, '.', Color::Gray);
            plot(cells, colors, cx + dx + 1, contact, '.', Color::Gray);
        }
    }

    for (dx, dy) in [(-8_i32, -5_i32), (-3, -7), (2, -6), (7, -4), (10, -1)] {
        let shimmer = ((app.visual_phase * 3.0) as i32 + dx.abs()) % 4;
        if shimmer < 2 {
            plot(cells, colors, cx + dx, cy + dy, '.', Color::White);
        }
    }
}

fn mouse_contact_y(dx: i32, center_y: i32) -> i32 {
    let body = 5.8 - (dx.abs() as f32 * 0.23).min(4.2);
    center_y + body.round() as i32
}

fn draw_mouse_wind_hits(
    cells: &mut [Vec<char>],
    colors: &mut [Vec<Color>],
    center_x: usize,
    center_y: usize,
    app: &App,
) {
    let cx = center_x as i32;
    let cy = center_y as i32;
    let drift = (app.visual_phase * 7.0) as i32;
    let force = 2 + (app.noise_amount * 5.0) as i32;

    for row in -5..=5 {
        let y = cy + row;
        let offset = (drift + row * 5).rem_euclid(18);
        for index in 0..force {
            let x = cx - 18 + ((offset + index * 5) % 36);
            let ch = if row.abs() < 2 { '>' } else { '~' };
            plot(cells, colors, x, y, ch, Color::Green);
            if index % 2 == 0 {
                plot(cells, colors, x - 1, y, '-', Color::Gray);
            }
        }
    }
}

fn hash_cell(x: i32, y: i32, seed: i32) -> u32 {
    let mut n = x
        .wrapping_mul(73_856_093)
        .wrapping_add(y.wrapping_mul(19_349_663))
        .wrapping_add(seed.wrapping_mul(83_492_791));
    n ^= n >> 13;
    n = n.wrapping_mul(1_274_126_177);
    (n ^ (n >> 16)).unsigned_abs()
}

fn draw_center(
    cells: &mut [Vec<char>],
    colors: &mut [Vec<Color>],
    center_x: usize,
    center_y: usize,
    app: &App,
) {
    let color = if app.gate_open {
        particle_color(app.waveform.next_color_index())
    } else {
        Color::Gray
    };
    let shade = if app.gate_open {
        particle_color(app.texture_color_index())
    } else {
        Color::DarkGray
    };
    let highlight = if app.gate_open {
        Color::White
    } else {
        Color::Gray
    };
    let bob = (app.visual_phase * 5.0).sin().round() as i32;
    let tail_right = (app.visual_phase * 1.4).sin() >= 0.0;
    let eye = if app.gate_open { 'O' } else { 'o' };

    let tail = if tail_right { "___" } else { "   " };
    let left_tail = if tail_right { "   " } else { "___" };
    let rows = [
        format!("{left_tail}  ()      ()   {tail}"),
        "       .-''''-.       ".to_string(),
        format!("     .'  {eye}  {eye} '.     "),
        "    /   .----.  \\    ".to_string(),
        "   |   /  __  \\  |   ".to_string(),
        "    \\  \\      / /    ".to_string(),
        "     '-.'----'.-'    ".to_string(),
        "       /_/  \\_\\      ".to_string(),
    ];

    let model_width = rows.iter().map(|row| row.len()).max().unwrap_or(0) as i32;
    let start_x = center_x as i32 - model_width / 2;
    let start_y = center_y as i32 - rows.len() as i32 / 2 + bob;

    for (row_index, row) in rows.iter().enumerate() {
        for (col_index, ch) in row.chars().enumerate() {
            if ch == ' ' {
                continue;
            }

            let x = start_x + col_index as i32;
            let y = start_y + row_index as i32;
            if y >= 0 && x >= 0 && y < cells.len() as i32 && x < cells[0].len() as i32 {
                cells[y as usize][x as usize] = ch;
                colors[y as usize][x as usize] = match ch {
                    'O' | 'o' => highlight,
                    '\'' | '.' | '_' | '-' => shade,
                    _ => color,
                };
            }
        }
    }
}

fn draw_3d_anchor_shadow(
    cells: &mut [Vec<char>],
    colors: &mut [Vec<Color>],
    center_x: usize,
    center_y: usize,
    app: &App,
) {
    let cx = center_x as i32;
    let cy = center_y as i32;
    let pulse = if app.gate_open { 2 } else { 0 };
    for dx in -11_i32..=11 {
        let edge = dx.abs();
        let ch = if edge > 9 {
            '.'
        } else if edge > 6 {
            '-'
        } else {
            '_'
        };
        plot(
            cells,
            colors,
            cx + dx,
            cy + 5 + pulse / 2,
            ch,
            Color::DarkGray,
        );
    }
    for dy in -5_i32..=4 {
        let width = 10 - dy.abs();
        if width > 0 {
            plot(cells, colors, cx - width, cy + dy, '.', Color::DarkGray);
            plot(cells, colors, cx + width, cy + dy, '.', Color::DarkGray);
        }
    }
    if !ratty::enabled() {
        let hint = "ratty 3d";
        let start = cx - hint.len() as i32 / 2;
        for (index, ch) in hint.chars().enumerate() {
            plot(cells, colors, start + index as i32, cy, ch, Color::Gray);
        }
    }
}

fn draw_ratty_mouse(frame: &mut Frame, area: Rect, app: &App) {
    if !ratty::enabled() || area.is_empty() {
        return;
    }

    if app.visual_mode != VisualMode::ThreeD {
        if let Some(cell) = frame.buffer_mut().cell_mut((area.x, area.y)) {
            let existing = cell.symbol().to_string();
            cell.set_symbol(&(ratty::mouse_delete_sequence() + &existing));
        }
        return;
    }

    let width = area.width.min(28).max(1);
    let height = area.height.min(14).max(1);
    let base_x = area.x + area.width.saturating_sub(width) / 2;
    let wind_offset = if app.noise_texture == NoiseTexture::Wind {
        ((app.visual_phase * 0.65).sin() * (app.noise_amount * 1.4)).round() as i32
    } else {
        0
    };
    let x = (base_x as i32 + wind_offset)
        .clamp(area.x as i32, area.right().saturating_sub(width) as i32);
    let y = (area.y as i32 + area.height.saturating_sub(height) as i32 / 2)
        .clamp(area.y as i32, area.bottom().saturating_sub(height) as i32);
    let object_area = Rect {
        x: x as u16,
        y: y as u16,
        width,
        height,
    };
    let color = mouse_rgb(app);
    let pulse = if app.gate_open {
        0.48
    } else {
        0.22 + app.noise_amount * 0.18
    };
    let breath = (app.visual_phase * 1.15).sin() * 0.035;
    let wet_glint = (app.visual_phase * 2.2).sin().max(0.0) * 0.055;
    let brightness = 0.62 + rain_force(app) * 0.2 + breath + wet_glint;
    let place = ratty::mouse_place_sequence(object_area, color, brightness, pulse, false);

    if let Some(cell) = frame.buffer_mut().cell_mut((object_area.x, object_area.y)) {
        let existing = cell.symbol().to_string();
        cell.set_symbol(&(place + &existing));
    }
}

fn footer_status_line(app: &App) -> Line<'static> {
    Line::from(vec![
        label("note "),
        value(
            format!("{} {:.1}hz", app.last_note, app.audio_frequency()),
            Color::Yellow,
        ),
        muted("  "),
        label("wave "),
        Span::styled(app.waveform.as_str(), wave_style(app.waveform)),
        muted("  "),
        label("noise "),
        Span::styled(app.noise_texture.as_str(), texture_style(app)),
        muted(" "),
        Span::styled(
            bar(app.noise_amount / 0.6, 8),
            Style::default().fg(particle_color(app.texture_color_index())),
        ),
        muted("  "),
        label("mouse "),
        Span::styled(app.visual_mode.as_str(), mode_style(app)),
        muted("  "),
        label("gate "),
        Span::styled(
            if app.gate_open { "open" } else { "idle" },
            if app.gate_open {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
        muted("  "),
        label("cutoff "),
        Span::styled(
            format!("{:.0}hz", app.cutoff_hz),
            Style::default().fg(Color::Magenta),
        ),
        muted("  "),
        label("audio "),
        Span::styled(
            compact_audio_status(&app.audio_status),
            if app.audio_status.contains("live") {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ),
    ])
}

fn scope_lines(app: &App) -> Vec<Line<'static>> {
    let top = app
        .meters
        .iter()
        .enumerate()
        .map(|(index, value)| {
            if index % 2 == 0 && *value > 0.18 {
                '^'
            } else if *value > 0.11 {
                '-'
            } else {
                ' '
            }
        })
        .collect::<String>();
    let bottom = app
        .meters
        .iter()
        .map(|value| match (value * 10.0) as u8 {
            0 => ' ',
            1 => '.',
            2 => ':',
            3 => '-',
            4 => '=',
            5 => '+',
            6 => '*',
            7 => '#',
            _ => '@',
        })
        .collect::<String>();

    vec![
        Line::from(Span::styled(top, Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(
            bottom,
            Style::default().fg(particle_color(app.texture_color_index())),
        )),
    ]
}

fn keyboard_lines(app: &App) -> Vec<Line<'static>> {
    let active = app.root_midi.rem_euclid(12) as usize;
    let mut key_line = Vec::new();
    let mut note_line = Vec::new();

    for (index, (key, note)) in KEYS.iter().enumerate() {
        let color = if index == active {
            Color::Yellow
        } else if note.contains('#') {
            Color::Gray
        } else {
            Color::DarkGray
        };
        let style = if index == active {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };
        key_line.push(Span::styled(format!(" {key:^3} "), style));
        note_line.push(Span::styled(format!(" {note:^3} "), style));
    }

    vec![Line::from(key_line), Line::from(note_line)]
}

fn label(text: impl Into<String>) -> Span<'static> {
    Span::styled(
        text.into(),
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )
}

fn value(text: impl Into<String>, color: Color) -> Span<'static> {
    Span::styled(
        text.into(),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )
}

fn muted(text: impl Into<String>) -> Span<'static> {
    Span::styled(text.into(), Style::default().fg(Color::DarkGray))
}

fn bar(value: f32, width: usize) -> String {
    let filled = (value.clamp(0.0, 1.0) * width as f32).round() as usize;
    format!("{}{}", "#".repeat(filled), ".".repeat(width - filled))
}

fn compact_audio_status(status: &str) -> String {
    if status.contains("live") {
        "live".to_string()
    } else {
        "silent".to_string()
    }
}

fn row_rect(area: Rect, row: usize) -> Rect {
    Rect {
        x: area.x,
        y: area.y + row as u16,
        width: area.width,
        height: 1,
    }
}

fn particle_color(index: usize) -> Color {
    match index % 4 {
        0 => Color::Cyan,
        1 => Color::LightRed,
        2 => Color::Yellow,
        _ => Color::Green,
    }
}

fn mode_color(app: &App) -> Color {
    match app.visual_mode {
        VisualMode::TwoD => Color::Gray,
        VisualMode::ThreeD => {
            if ratty::enabled() {
                Color::Cyan
            } else {
                Color::Yellow
            }
        }
    }
}

fn mode_style(app: &App) -> Style {
    Style::default()
        .fg(mode_color(app))
        .add_modifier(Modifier::BOLD)
}

fn mouse_rgb(app: &App) -> [u8; 3] {
    match app.noise_texture {
        NoiseTexture::Wind => [126, 166, 168],
        _ => [88, 122, 146],
    }
}

fn rain_force(app: &App) -> f32 {
    let weather = match app.noise_texture {
        NoiseTexture::Wind => 0.82,
        NoiseTexture::Rain => 0.9,
        NoiseTexture::White => 0.7,
        NoiseTexture::Brown => 0.62,
        NoiseTexture::None => 0.5,
    };
    ((0.5 + app.noise_amount * 0.85) * weather).clamp(0.34, 0.9)
}

fn rain_slant(app: &App) -> i32 {
    if app.noise_texture == NoiseTexture::Wind {
        1 + (app.noise_amount * 3.0).round() as i32
    } else {
        0
    }
}

fn plot(
    cells: &mut [Vec<char>],
    colors: &mut [Vec<Color>],
    x: i32,
    y: i32,
    ch: char,
    color: Color,
) {
    if y >= 0 && x >= 0 && y < cells.len() as i32 && x < cells[0].len() as i32 {
        cells[y as usize][x as usize] = ch;
        colors[y as usize][x as usize] = color;
    }
}

fn wave_style(waveform: Waveform) -> Style {
    Style::default()
        .fg(particle_color(waveform.next_color_index()))
        .add_modifier(Modifier::BOLD)
}

fn texture_style(app: &App) -> Style {
    Style::default()
        .fg(particle_color(app.texture_color_index()))
        .add_modifier(Modifier::BOLD)
}
