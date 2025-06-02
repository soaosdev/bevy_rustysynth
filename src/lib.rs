#![warn(missing_docs)]
//! A plugin which adds MIDI file and soundfont audio support to the [bevy](https://crates.io/crates/bevy) engine via [rustysynth](https://crates.io/crates/rustysynth).

#[cfg(all(feature = "bevy_audio", feature = "kira"))]
compile_error!("Cannot compile with both bevy_audio and kira features enabled simultaneously. Please disable one of these features");

#[cfg(feature = "bevy_audio")]
use bevy::audio::AddAudioSource;
use bevy::prelude::*;
use lazy_static::lazy_static;
use rustysynth::SoundFont;
#[cfg(feature = "hl4mgm")]
use std::io::Cursor;
use std::{
    fs::File,
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
};

mod assets;
pub use assets::*;

#[cfg(feature = "hl4mgm")]
pub(crate) static HL4MGM: &[u8] = include_bytes!("./embedded_assets/hl4mgm.sf2");

lazy_static! {
    pub(crate) static ref DEFAULT_SOUNDFONT: Arc<Mutex<Option<Arc<SoundFont>>>> =
        Arc::new(Mutex::new(None));
    pub(crate) static ref SOUNDFONT: Arc<Mutex<Option<Arc<SoundFont>>>> =
        Arc::new(Mutex::new(None));
}

/// Set labels for rustysynth systems
#[derive(SystemSet, Hash, Clone, PartialEq, Eq, Debug)]
pub enum RustySynthSet {
    /// Rustysynth systems used for setup
    Setup,
    /// Rustysynth systems used during the update loop
    Update,
}

/// This plugin configures the soundfont used for playback and registers MIDI assets.
#[derive(Debug)]
pub struct RustySynthPlugin<R: Read + Clone + 'static> {
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
        *DEFAULT_SOUNDFONT.lock().unwrap() = Some(Arc::new(
            SoundFont::new(&mut self.soundfont.clone()).unwrap(),
        ));
        info!("Setting Soundfont Initially");
        *SOUNDFONT.lock().unwrap() = DEFAULT_SOUNDFONT.lock().unwrap().clone();
        app.init_asset_loader::<MidiAssetLoader>()
            .add_event::<SetSoundfontEvent>()
            .add_systems(Startup, handle_set_soundfont.in_set(RustySynthSet::Setup))
            .add_systems(Update, handle_set_soundfont.in_set(RustySynthSet::Update));
        #[cfg(feature = "bevy_audio")]
        app.init_asset::<MidiAudio>()
            .add_audio_source::<MidiAudio>();
    }
}

pub(crate) fn set_soundfont<R: Read + 'static>(mut reader: R) {
    info!("Setting Soundfont");
    *SOUNDFONT.lock().unwrap() = Some(Arc::new(SoundFont::new(&mut reader).unwrap()));
}

/// Event for setting the soundfont after initialization
/// This will not affect sounds which have already been rendered
#[derive(Event)]
pub enum SetSoundfontEvent {
    /// Load soundfont from bytes
    Bytes(Vec<u8>),
    /// Load soundfont at path
    Path(PathBuf),
    /// Load default soundfont
    Default,
}

fn handle_set_soundfont(mut event_reader: EventReader<SetSoundfontEvent>) {
    for event in event_reader.read() {
        match event {
            SetSoundfontEvent::Bytes(items) => {
                set_soundfont(Cursor::new(items.clone()));
            }
            SetSoundfontEvent::Path(path_buf) => {
                set_soundfont(File::open(path_buf).unwrap());
            }
            SetSoundfontEvent::Default => {
                *SOUNDFONT.lock().unwrap() = DEFAULT_SOUNDFONT.lock().unwrap().clone();
            }
        }
    }
}
