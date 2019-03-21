use hound;
use std;

//==================================================================================================
// AudioContext
//==================================================================================================
//
// TODO: Set this back to 44100 when we finished the update to SDL2
const AUDIO_SAMPLE_RATE_HZ: usize = 48000;
const AUDIO_CHANNELS: usize = 2;

pub enum SoundStartTime {
    Immediately,
    OnNextMeasure,
    OnNextBeat,
    OnNextHalfBeat,
    OnNextQuarterBeat,
}

pub struct Sound {
    start_time: SoundStartTime,
    start_frame_index: Option<usize>,
    end_frame_index: Option<usize>,
}

#[derive(Default)]
pub struct AudioContext {
    pub next_uncommitted_frame_index: usize,

    pub num_channels: usize,
    pub sample_rate_hz: usize,

    pongi_test_sound_samples: Vec<f32>,
    pongi_test_music_samples: Vec<f32>,

    sounds: Vec<Sound>,
}

impl AudioContext {
    pub fn new(num_channels: usize, sample_rate_hz: usize) -> AudioContext {
        if num_channels != AUDIO_CHANNELS || sample_rate_hz != AUDIO_SAMPLE_RATE_HZ {
            unimplemented!();
        }

        AudioContext {
            num_channels,
            sample_rate_hz,
            ..Default::default()
        }
    }

    pub fn play_debug_sound(&mut self, start_time: SoundStartTime) {
        self.sounds.push(Sound {
            start_time,
            start_frame_index: None,
            end_frame_index: None,
        });
    }

    pub fn fill_buffer(&mut self, audio_output_buffer: &mut Vec<f32>) {
        let sample_rate_hz = self.sample_rate_hz;
        let sample_length_sec: f64 = 1.0 / sample_rate_hz as f64;
        let samples_buffer_len = self.num_channels * sample_rate_hz * 4 / 60; // ~ 4 Frames @60Hz

        // Update audio_output_buffer
        let num_committed_frames =
            (samples_buffer_len - audio_output_buffer.len()) / self.num_channels;
        self.next_uncommitted_frame_index += num_committed_frames;
        // NOTE: We clear the entire vector as we want to overwrite the uncommited samples anyway,
        //       if there where any.
        audio_output_buffer.clear();

        let next_uncommitted_frame_index = self.next_uncommitted_frame_index;
        // Test sound output
        const NOTE_A_HZ: f64 = 440.0;
        let num_frames_to_commit = samples_buffer_len / 2;
        let mut debug_sine_time = next_uncommitted_frame_index as f64 * sample_length_sec as f64;

        for _ in 0..num_frames_to_commit {
            let sine_amplitude =
                0.2 * f64::sin(NOTE_A_HZ * debug_sine_time * 2.0 * std::f64::consts::PI);

            debug_sine_time += sample_length_sec as f64;

            // Stereo
            audio_output_buffer.push(sine_amplitude as f32);
            audio_output_buffer.push(sine_amplitude as f32);
        }

        self.sounds.retain(|sound| {
            if let Some(frame_index) = sound.end_frame_index {
                frame_index >= next_uncommitted_frame_index
            } else {
                true
            }
        });

        for sound in self.sounds.iter_mut() {
            if sound.start_frame_index.is_none() {
                sound.start_frame_index = Some(next_uncommitted_frame_index);
                sound.end_frame_index =
                    Some(next_uncommitted_frame_index + self.pongi_test_sound_samples.len());
            }

            let vec_start_index = next_uncommitted_frame_index - sound.start_frame_index.unwrap();

            for index in vec_start_index
                ..usize::min(
                    audio_output_buffer.len() / self.num_channels,
                    self.pongi_test_sound_samples.len(),
                )
            {
                audio_output_buffer[2 * (index - vec_start_index) + 0] =
                    self.pongi_test_sound_samples[index];
                audio_output_buffer[2 * (index - vec_start_index) + 1] =
                    self.pongi_test_sound_samples[index];
            }
        }
    }

    pub fn reinitialize(&mut self) {
        let reader =
            hound::WavReader::open("data/pongi_blip.wav").expect("Could not load test sound");
        let num_samples = reader.len();

        let samples: Vec<_> = reader
            .into_samples::<i16>()
            .filter_map(Result::ok)
            .map(integer_sample_to_float)
            .collect();

        debug_assert!(samples.len() == num_samples as usize);

        self.pongi_test_sound_samples = samples;
    }
}

fn integer_sample_to_float(sample: i16) -> f32 {
    sample as f32 / std::i16::MAX as f32
}
