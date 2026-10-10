use rodio::source::Source;
use rodio::stream::DeviceSinkBuilder;
use rodio::{ChannelCount, MixerDeviceSink, Player, SampleRate};

struct XoSource {
    sample_rate: SampleRate,
    pattern: [u8; 16],
    index: usize,
    current_value: f32,
}

impl XoSource {
    const MULTIPLIER: usize = 128;

    fn new(pitch: u8, pattern: [u8; 16]) -> Self {
        let frequency = 4000.0 * 2f32.powf((pitch as f32 - 64.0) / 48.0);
        let sample_rate = SampleRate::new((Self::MULTIPLIER as f32 * frequency) as u32).unwrap();

        XoSource {
            sample_rate,
            pattern,
            index: 0,
            current_value: -0.1,
        }
    }

    fn bit_is_set(pattern: &[u8; 16], bit: usize) -> bool {
        let index = bit / 8;
        let shift = 7 - bit % 8;

        (pattern[index] >> shift) & 1 == 1
    }
}

impl Source for XoSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        ChannelCount::new(1).unwrap()
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl Iterator for XoSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index.is_multiple_of(Self::MULTIPLIER) {
            let bit = self.index / Self::MULTIPLIER;

            self.current_value = if Self::bit_is_set(&self.pattern, bit) {
                0.1
            } else {
                -0.1
            };
        }

        self.index += 1;

        if self.index >= 128 * Self::MULTIPLIER {
            self.index = 0;
        }

        Some(self.current_value)
    }
}

struct SinkPlayer {
    _sink: MixerDeviceSink,
    player: Player,
}

pub struct Ca555 {
    beep: Option<SinkPlayer>,
    st: u8,
    pattern: [u8; 16],
    pitch: u8,
}

impl Ca555 {
    pub fn new() -> Self {
        let sink = DeviceSinkBuilder::open_default_sink();

        if let Ok(mut sink) = sink {
            let player = Player::connect_new(sink.mixer());

            sink.log_on_drop(false);

            Self {
                beep: Some(SinkPlayer {
                    _sink: sink,
                    player,
                }),
                st: 0,
                pattern: [0xf0; 16],
                pitch: 64,
            }
        } else {
            Self {
                beep: None,
                st: 0,
                pattern: [0; 16],
                pitch: 0,
            }
        }
    }

    pub fn set(&mut self, st: u8) {
        self.st = st;
    }

    pub fn tick(&mut self) {
        if let Some(sink) = &self.beep {
            let samples = if self.st == 0 {
                [0u8; 16]
            } else {
                self.pattern
            };

            let sound = XoSource::new(self.pitch, samples)
                .take_duration(std::time::Duration::from_micros(1_000_000 / 60));

            sink.player.append(sound);
        }

        if self.st > 0 {
            self.st -= 1;
        }
    }

    pub fn pattern(&mut self, pattern: [u8; 16]) {
        self.pattern = pattern;
    }

    pub fn pitch(&mut self, pitch: u8) {
        self.pitch = pitch;
    }
}

impl Default for Ca555 {
    fn default() -> Self {
        Self::new()
    }
}
