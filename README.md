# bevy_rustysynth

![Crates](https://img.shields.io/crates/v/bevy_rustysynth)
![License](https://img.shields.io/badge/license-0BSD%2FMIT%2FApache-blue.svg)

A plugin which adds MIDI file and soundfont audio support to the bevy engine via rustysynth.

From version 0.4, the crate has undergone significant rewrites, and now works with the default `bevy_audio` backend (`bevy_audio` feature) OR [`bevy_kira_audio`](https://github.com/NiklasEi/bevy_kira_audio) (`kira` feature)

## Compatibility

| Crate Version | Bevy Version |
| ------------- | ------------ |
| 0.5           | 0.15         |
| 0.2           | 0.14         |

## Installation

### crates.io
```toml
[dependencies]
bevy_rustysynth = "0.5"
```

### Using git URL in Cargo.toml
```toml
[dependencies.bevy_rustysynth]
git = "https://git.soaos.dev/soaos/bevy_rustysynth.git"
```

## Usage

In `main.rs`:
```rs
use bevy::prelude::*;
use bevy_rustysynth::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            RustySynthPlugin {
                soundfont: // Bring your own soundfont or enable the "hl4mgm" feature to use a terrible 4MB default
            }
        ))
        .run();
}
```
Then you can load and play a MIDI like any other audio file:

### `bevy_audio` Example
```rs
let midi_handle = asset_server.load::<MidiAudioSource>("example.mid");

commands.spawn(AudioSourceBundle {
    source: midi_handle,
    ..Default::default()
});
```

### `bevy_kira_audio` Example
```rs
let midi_handle = asset_server.load::<AudioSource>("example.mid");

audio.play(midi_handle);
```

## License

This crate is licensed under your choice of 0BSD, Apache-2.0, or MIT license.

