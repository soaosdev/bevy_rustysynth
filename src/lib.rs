#![warn(missing_docs)]
//! A plugin which adds MIDI file and soundfont audio support to the [bevy](https://crates.io/crates/bevy) engine via [rustysynth](https://crates.io/crates/rustysynth).

#[cfg(all(feature = "bevy_audio", feature = "kira"))]
compile_error!("Cannot compile with both bevy_audio and kira features enabled simultaneously. Please disable one of these features");

use bevy::prelude::*;
use rustysynth::SoundFont;
use std::{
    io::Read,
    sync::{Arc, OnceLock},
};
#[cfg(feature = "hl4mgm")]
use std::io::Cursor;
#[cfg(feature = "bevy_audio")]
use bevy::audio::AddAudioSource;


mod assets;
pub use assets::*;

#[cfg(feature = "hl4mgm")]
pub(crate) static HL4MGM: &[u8] = include_bytes!("./embedded_assets/hl4mgm.sf2");

pub(crate) static SOUNDFONT: OnceLock<Arc<SoundFont>> = OnceLock::new();

/// This plugin configures the soundfont used for playback and registers MIDI assets.
#[derive(Debug)]
pub struct RustySynthPlugin<R: Read + Send + Sync + Clone + 'static> {
    /// Reader for soundfont data.
    pub soundfont: R,
}

#[cfg(feature = "hl4mgm")]
impl Default for RustySynthPlugin<Cursor<&[u8]>> {
    fn default() -> Self {
        Self {
            soundfont: Cursor::new(HL4MGM),
        }
    }
}

impl<R: Read + Send + Sync + Clone + 'static> Plugin for RustySynthPlugin<R> {
    fn build(&self, app: &mut App) {
        let _ = SOUNDFONT.set(Arc::new(
            SoundFont::new(&mut self.soundfont.clone()).unwrap(),
        ));
        app.init_asset_loader::<MidiAssetLoader>();
        #[cfg(feature = "bevy_audio")]
        app.init_asset::<MidiAudio>().add_audio_source::<MidiAudio>();
    }
}
