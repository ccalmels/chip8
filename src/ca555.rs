use crate::cpu::Beeper;
use rodio::source::{SineWave, Source};
use rodio::stream::DeviceSinkBuilder;
use rodio::{MixerDeviceSink, Player};

struct SinkPlayer {
    _sink: MixerDeviceSink,
    player: Player,
}

pub struct Ca555 {
    beep: Option<SinkPlayer>,
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
            }
        } else {
            Self { beep: None }
        }
    }
}

impl Default for Ca555 {
    fn default() -> Self {
        Self::new()
    }
}

impl Beeper for Ca555 {
    fn beep(&self, running: bool) {
        if let Some(beep) = &self.beep {
            if running {
                beep.player.play();
            } else {
                beep.player.pause();
            }
        }
    }
}
