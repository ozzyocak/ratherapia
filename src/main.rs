mod app;
mod audio;
mod engine;
mod ratty;
mod ui;
mod video;

use std::io;
use std::io::Write;
use std::time::{Duration, Instant};

use anyhow::Result;
use app::{App, NoiseTexture, VisualMode, Waveform};
use audio::AudioEngine;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

fn main() -> Result<()> {
    if let Some(output) = video_output_arg() {
        video::render_demo(output)?;
        return Ok(());
    }

    let audio = AudioEngine::new();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    if ratty::enabled() {
        ratty::register_mouse(&mut stdout)?;
    }

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = App::new(audio.control());
    if ratty::enabled() {
        app.visual_mode = VisualMode::ThreeD;
    }

    let result = run(&mut terminal, &mut app);

    disable_raw_mode()?;
    if ratty::enabled() {
        ratty::clear_mouse(terminal.backend_mut())?;
    }
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.backend_mut().flush()?;
    terminal.show_cursor()?;

    result
}

fn video_output_arg() -> Option<String> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("render-video") => Some(
            args.next()
                .unwrap_or_else(|| "renders/ratherapia.avi".to_string()),
        ),
        _ => None,
    }
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let tick_rate = Duration::from_millis(16);
    let mut last_tick = Instant::now();

    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, app))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let previous_visual_mode = app.visual_mode;
                    handle_key(app, key.code);
                    if ratty::enabled() && previous_visual_mode != app.visual_mode {
                        match app.visual_mode {
                            VisualMode::ThreeD => ratty::register_mouse(terminal.backend_mut())?,
                            VisualMode::TwoD => ratty::clear_mouse(terminal.backend_mut())?,
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            let dt = last_tick.elapsed().as_secs_f32();
            app.tick(dt);
            last_tick = Instant::now();
        }
    }

    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char(' ') => app.toggle_gate(),
        KeyCode::Char('1') => app.set_waveform(Waveform::Sine),
        KeyCode::Char('2') => app.set_waveform(Waveform::Square),
        KeyCode::Char('3') => app.set_waveform(Waveform::Saw),
        KeyCode::Char('4') => app.set_waveform(Waveform::Triangle),
        KeyCode::Char('5') => app.set_noise_texture(NoiseTexture::None),
        KeyCode::Char('6') => app.set_noise_texture(NoiseTexture::White),
        KeyCode::Char('7') => app.set_noise_texture(NoiseTexture::Brown),
        KeyCode::Char('8') => app.set_noise_texture(NoiseTexture::Rain),
        KeyCode::Char('9') => app.set_noise_texture(NoiseTexture::Wind),
        KeyCode::Char('0') => app.toggle_visual_mode(),
        KeyCode::Up => app.shift_note(1),
        KeyCode::Down => app.shift_note(-1),
        KeyCode::Right => app.adjust_cutoff(60.0),
        KeyCode::Left => app.adjust_cutoff(-60.0),
        KeyCode::Char('+') | KeyCode::Char('=') => app.adjust_gain(0.03),
        KeyCode::Char('-') => app.adjust_gain(-0.03),
        KeyCode::Char(']') => app.adjust_noise_amount(0.03),
        KeyCode::Char('[') => app.adjust_noise_amount(-0.03),
        KeyCode::Char('z') => app.play_note(0),
        KeyCode::Char('s') => app.play_note(1),
        KeyCode::Char('x') => app.play_note(2),
        KeyCode::Char('d') => app.play_note(3),
        KeyCode::Char('c') => app.play_note(4),
        KeyCode::Char('v') => app.play_note(5),
        KeyCode::Char('g') => app.play_note(6),
        KeyCode::Char('b') => app.play_note(7),
        KeyCode::Char('h') => app.play_note(8),
        KeyCode::Char('n') => app.play_note(9),
        KeyCode::Char('j') => app.play_note(10),
        KeyCode::Char('m') => app.play_note(11),
        _ => {}
    }
}
