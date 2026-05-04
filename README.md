# ratherapia

ratherapia is a calming Rust terminal synth built with Ratatui, CPAL, and
Ratty. It blends ambient sound, rain textures, reactive terminal particles, and
an optional inline 3D rat scene into a quiet TUI instrument.

## Features

- Ratatui/Crossterm terminal interface
- CPAL oscillator synth with sine, square, saw, and triangle waveforms
- Key-triggered pluck sound effects
- Procedural white noise, brown noise, rain, and wind textures
- Bevy ECS particle simulation rendered as a terminal effect
- Center-stage pseudo-3D mouse figure with optional Ratty inline 3D object support
- Silent fallback when the current machine has no usable audio output backend

## Install

```sh
cargo install ratherapia
ratherapia
```

## Run From Source

```sh
cargo run
```

## Optional Ratty 3D Mode

Ratty is a separate terminal emulator that understands inline 3D objects through
the Ratty Graphics Protocol. To emit the center mouse as a Ratty 3D object while
keeping the normal Ratatui fallback visible, install Ratty and run:

```sh
cargo install --git https://github.com/orhun/ratty
cargo install --git https://github.com/ozzyocak/ratherapia
```

Open Ratty, then run Ratherapia inside the Ratty terminal:

```sh
ratherapia --ratty
```

From source, open Ratty, `cd` into the project directory, then run:

```sh
cargo run -- --ratty
```

The app registers the bundled `SpinyMouse.glb` model with Ratty from
`assets/objects`, so the polished 3D mouse is used instead of the rough OBJ
fallback. Regular terminals should use plain `ratherapia` or `cargo run`.

## Render a Ratatui Video

This project can also render the Ratatui interface headlessly into an
uncompressed AVI file using only Rust code in the app.

```sh
cargo run -- render-video renders/ratherapia.avi
```

The prototype renders a scripted 6-second, 30 fps demo at 880x448. The output is
large because it is raw video, but it proves the Ratatui buffer -> raster frame
-> AVI pipeline without Remotion, a browser, or FFmpeg.

## Controls

- `z s x d c v g b h n j m`: play chromatic notes
- `space`: toggle the sustained gate
- `1 2 3 4`: switch waveform
- `5 6 7 8 9`: off, white noise, brown noise, rain, wind
- `0`: toggle the center mouse between 2D ASCII and Ratty 3D modes
- `[` and `]`: adjust noise texture level
- `up/down`: shift pitch
- `left/right`: adjust filter cutoff
- `+/-`: adjust gain
- `q` or `esc`: quit

## License

MIT
