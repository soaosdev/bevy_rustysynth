use bevy::asset::{io::Reader, AssetLoader, LoadContext};
#[cfg(feature = "bevy_audio")]
use bevy::prelude::*;
use itertools::Itertools;
use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};
#[cfg(feature = "kira")]
use std::future::Future;
use std::{
    io::{self, Cursor},
    sync::Arc,
    time::Duration,
};

use crate::SOUNDFONT;

/// Represents a single MIDI note in a sequence
#[derive(Clone, Debug)]
pub struct MidiNote {
    /// Channel to play the note on
    pub channel: i32,
    /// Preset (instrument) to play the note with (see GM spec.)
    pub preset: i32,
    /// Bank to play note with
    pub bank: i32,
    /// Key to play (60 is middle C)
    pub key: i32,
    /// Velocity to play note at
    pub velocity: i32,
    /// Duration to play note for
    pub duration: Duration,
}

impl Default for MidiNote {
    fn default() -> Self {
        Self {
            channel: 0,
            preset: 0,
            bank: 0,
            key: 60,
            velocity: 100,
            duration: Duration::from_secs(1),
        }
    }
}

/// AssetLoader for MIDI files (.mid/.midi)
#[derive(Default, Debug)]
pub struct MidiAssetLoader;

/// Decoder for MIDI file playback
pub struct MidiFileDecoder {
    sample_rate: usize,
    data: Vec<f32>,
    #[cfg(feature = "bevy_audio")]
    index: usize,
}

impl MidiFileDecoder {
    /// Construct and render a MIDI sequence with the given MIDI data and soundfont.
    pub fn new(midi_data: Vec<u8>, soundfont: Arc<SoundFont>) -> Self {
        let sample_rate = 44100_usize;
        let settings = SynthesizerSettings::new(sample_rate as i32);
        let synthesizer =
            Synthesizer::new(&soundfont, &settings).expect("Failed to create synthesizer.");

        let mut data = Vec::new();
        let mut sequencer = MidiFileSequencer::new(synthesizer);
        let mut midi_data = Cursor::new(midi_data);
        let midi = Arc::new(MidiFile::new(&mut midi_data).expect("Failed to read midi file."));
        sequencer.play(&midi, false);
        let mut left: Vec<f32> = vec![0_f32; sample_rate];
        let mut right: Vec<f32> = vec![0_f32; sample_rate];
        while !sequencer.end_of_sequence() {
            sequencer.render(&mut left, &mut right);
            for value in left.iter().interleave(right.iter()) {
                data.push(*value);
            }
        }
        Self {
            sample_rate,
            data,
            #[cfg(feature = "bevy_audio")]
            index: 0,
        }
    }

    /// Render a MIDI sequence with the given soundfont.
    pub fn new_sequence(midi_sequence: Vec<MidiNote>, soundfont: Arc<SoundFont>) -> Self {
        let sample_rate = 44100_usize;
        let settings = SynthesizerSettings::new(sample_rate as i32);
        let mut synthesizer =
            Synthesizer::new(&soundfont, &settings).expect("Failed to create synthesizer.");

        let mut data = Vec::new();

        for MidiNote {
            channel,
            preset,
            bank,
            key,
            velocity,
            duration,
        } in midi_sequence.iter()
        {
            synthesizer.process_midi_message(*channel, 0xB0, 0x00, *bank);
            synthesizer.process_midi_message(*channel, 0xC0, *preset, 0);
            synthesizer.note_on(*channel, *key, *velocity);
            let note_length = (sample_rate as f32 * duration.as_secs_f32()) as usize;
            let mut left: Vec<f32> = vec![0_f32; note_length];
            let mut right: Vec<f32> = vec![0_f32; note_length];
            for (left, right) in left
                .chunks_mut(sample_rate)
                .zip(right.chunks_mut(sample_rate))
            {
                synthesizer.render(left, right);
                for value in left.iter().interleave(right.iter()) {
                    data.push(*value);
                }
            }
            synthesizer.note_off(*channel, *key);
        }
        Self {
            sample_rate,
            data,
            #[cfg(feature = "bevy_audio")]
            index: 0,
        }
    }
}

#[cfg(all(feature = "bevy_audio", not(feature = "kira")))]
/// Asset containing MIDI file data to be used as a `Decodable` audio source
#[derive(Asset, TypePath, Debug)]
pub struct MidiAudio(Vec<u8>);

#[cfg(all(feature = "bevy_audio", not(feature = "kira")))]
mod bevy_audio {
    use super::*;
    use bevy::audio::{Decodable, Source};

    impl Source for MidiFileDecoder {
        fn current_frame_len(&self) -> Option<usize> {
            None
        }

        fn channels(&self) -> u16 {
            2
        }

        fn sample_rate(&self) -> u32 {
            self.sample_rate as u32
        }

        fn total_duration(&self) -> Option<std::time::Duration> {
            None
        }
    }

    impl Decodable for MidiAudio {
        type Decoder = MidiFileDecoder;

        type DecoderItem = <MidiFileDecoder as Iterator>::Item;

        fn decoder(&self) -> Self::Decoder {
            MidiFileDecoder::new(
                self.0.clone(),
                SOUNDFONT.lock().unwrap().as_ref().unwrap().clone(),
            )
        }
    }

    impl AssetLoader for MidiAssetLoader {
        type Asset = MidiAudio;

        type Settings = ();

        type Error = io::Error;

        async fn load(
            &self,
            reader: &mut dyn Reader,
            _settings: &Self::Settings,
            _load_context: &mut LoadContext<'_>,
        ) -> Result<Self::Asset, Self::Error> {
            let mut bytes = vec![];
            reader.read_to_end(&mut bytes).await?;
            Ok(MidiAudio(bytes))
        }

        fn extensions(&self) -> &[&str] {
            &["mid", "midi"]
        }
    }

    impl Iterator for MidiFileDecoder {
        type Item = f32;

        fn next(&mut self) -> Option<Self::Item> {
            let result = self.data.get(self.index).copied();
            self.index += 1;
            result
        }
    }
}

#[cfg(all(feature = "kira", not(feature = "bevy_audio")))]
/// Extensions For Rendering MIDI Audio
pub trait MidiAudioExtensions {
    /// Renders MIDI audio from orovided data
    fn from_midi_file(data: Vec<u8>) -> impl Future<Output = Self> + Send;
    /// Renders MIDI audio from provided note sequence
    fn from_midi_sequence(sequence: Vec<MidiNote>) -> impl Future<Output = Self> + Send;
}

#[cfg(all(feature = "kira", not(feature = "bevy_audio")))]
mod kira {
    use super::*;
    use bevy_kira_audio::{
        prelude::{Frame, StaticSoundData, StaticSoundSettings},
        AudioSource,
    };

    impl AssetLoader for MidiAssetLoader {
        type Asset = AudioSource;

        type Settings = ();

        type Error = io::Error;

        async fn load(
            &self,
            reader: &mut dyn Reader,
            _settings: &Self::Settings,
            _load_context: &mut LoadContext<'_>,
        ) -> Result<Self::Asset, Self::Error> {
            let mut bytes = vec![];
            reader.read_to_end(&mut bytes).await?;
            Ok(AudioSource::from_midi_file(bytes).await)
        }

        fn extensions(&self) -> &[&str] {
            &["mid", "midi"]
        }
    }

    impl MidiAudioExtensions for AudioSource {
        async fn from_midi_file(data: Vec<u8>) -> Self {
            let decoder =
                MidiFileDecoder::new(data, SOUNDFONT.lock().unwrap().as_ref().unwrap().clone());
            let frames = decoder
                .data
                .chunks(2)
                .map(|sample| Frame::new(sample[0], sample[1]))
                .collect::<Arc<[_]>>();
            let sample_rate = decoder.sample_rate as u32;
            let settings = StaticSoundSettings {
                ..Default::default()
            };
            AudioSource {
                sound: StaticSoundData {
                    sample_rate,
                    frames,
                    settings,
                    slice: None,
                },
            }
        }

        async fn from_midi_sequence(sequence: Vec<MidiNote>) -> Self {
            let decoder = MidiFileDecoder::new_sequence(
                sequence,
                SOUNDFONT.lock().unwrap().as_ref().unwrap().clone(),
            );
            let frames = decoder
                .data
                .chunks(2)
                .map(|sample| Frame::new(sample[0], sample[1]))
                .collect::<Arc<[_]>>();
            let sample_rate = decoder.sample_rate as u32;
            let settings = StaticSoundSettings {
                ..Default::default()
            };
            AudioSource {
                sound: StaticSoundData {
                    sample_rate,
                    frames,
                    settings,
                    slice: None,
                },
            }
        }
    }
}
