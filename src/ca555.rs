use rodio::source::{SineWave, Source};
use rodio::stream::DeviceSinkBuilder;
use rodio::{MixerDeviceSink, Player};

struct SinkPlayer {
    _sink: MixerDeviceSink,
    player: Player,
}

pub struct Ca555 {
    beep: Option<SinkPlayer>,
    st: u8,
}

impl Ca555 {
    pub fn new() -> Self {
        let sink = DeviceSinkBuilder::open_default_sink();

        if let Ok(mut sink) = sink {
            let player = Player::connect_new(sink.mixer());
            let sound = SineWave::new(440.0).amplify(0.10);

            sink.log_on_drop(false);

            player.pause();
            player.append(sound);

            Self {
                beep: Some(SinkPlayer {
                    _sink: sink,
                    player,
                }),
                st: 0,
            }
        } else {
            Self { beep: None, st: 0 }
        }
    }

    pub fn set(&mut self, st: u8) {
        self.st = st;
    }

    pub fn tick(&mut self) {
        if let Some(sink) = &self.beep {
            if self.st == 0 {
                sink.player.pause();
            } else {
                sink.player.play();
            }
        }

        if self.st > 0 {
            self.st -= 1;
        }
    }
}

impl Default for Ca555 {
    fn default() -> Self {
        Self::new()
    }
}
