use rodio::buffer::SamplesBuffer;
use rodio::source::Source;
use rodio::stream::DeviceSinkBuilder;
use rodio::{ChannelCount, MixerDeviceSink, Player, SampleRate};

struct SinkPlayer {
    _sink: MixerDeviceSink,
    player: Player,
}

pub struct Ca555 {
    beep: Option<SinkPlayer>,
    st: u8,
    pattern: [u8; 16],
    frequency: u32,
}

impl Ca555 {
    const MULTIPLIER: u32 = 100;

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
                frequency: 4000 * Self::MULTIPLIER,
            }
        } else {
            Self {
                beep: None,
                st: 0,
                pattern: [0; 16],
                frequency: 0,
            }
        }
    }

    fn bit_is_set(pattern: &[u8; 16], bit: usize) -> bool {
        let index = bit / 8;
        let shift = 7 - bit % 8;

        pattern[index] & (1 << shift) != 0
    }

    pub fn set(&mut self, st: u8) {
        self.st = st;
    }

    pub fn tick(&mut self) {
        if let Some(sink) = &self.beep {
            let samples = if self.st == 0 {
                [0f32; 12800]
            } else {
                let mut s = [0f32; 12800];

                for (i, sample) in s.iter_mut().enumerate() {
                    let i = i / Self::MULTIPLIER as usize;

                    *sample = if Self::bit_is_set(&self.pattern, i) {
                        0.1
                    } else {
                        -0.1
                    };
                }

                s
            };

            let sound = SamplesBuffer::new(
                ChannelCount::new(1).unwrap(),
                SampleRate::new(self.frequency).unwrap(),
                samples,
            )
            .repeat_infinite()
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
        let frequency = 4000.0 * 2f32.powf((pitch as f32 - 64.0) / 48.0);

        self.frequency = (frequency * Self::MULTIPLIER as f32) as u32;
    }
}

impl Default for Ca555 {
    fn default() -> Self {
        Self::new()
    }
}
