use std::time::Duration;

pub trait Tickable {
    type Error;

    const DT_TICK: Duration;
    const DT_TICK_TIMER: Duration;

    fn tick(&mut self) -> Result<(), Self::Error>;
    fn tick_timer(&mut self);
}

pub struct Ticker {
    accumulator: Duration,
    timer_acccumulator: Duration,
}

impl Ticker {
    pub fn new() -> Self {
        Ticker {
            accumulator: Duration::ZERO,
            timer_acccumulator: Duration::ZERO,
        }
    }

    pub fn tick<T: Tickable>(&mut self, delta: Duration, target: &mut T) -> Result<(), T::Error> {
        const {
            assert!(T::DT_TICK.as_nanos() < T::DT_TICK_TIMER.as_nanos());
        }
        self.accumulator += delta;

        while self.accumulator >= T::DT_TICK {
            self.accumulator -= T::DT_TICK;
            self.timer_acccumulator += T::DT_TICK;

            if self.timer_acccumulator >= T::DT_TICK_TIMER {
                target.tick_timer();

                self.timer_acccumulator -= T::DT_TICK_TIMER;
            }

            target.tick()?;
        }

        Ok(())
    }
}

impl Default for Ticker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tick_tests {
    use super::*;

    const DT_HZ: u64 = 500;
    const DT_TIMER_HZ: u64 = 60;

    #[derive(PartialEq, Debug)]
    struct MockError();

    struct MockTickable {
        tick: usize,
        tick_timer: usize,
        ticks: Vec<u8>,
        do_error: bool,
    }

    impl MockTickable {
        fn new() -> Self {
            MockTickable {
                tick: 0,
                tick_timer: 0,
                ticks: vec![],
                do_error: false,
            }
        }
    }

    impl Tickable for MockTickable {
        type Error = MockError;

        const DT_TICK: Duration = Duration::from_micros(1_000_000 / DT_HZ);
        const DT_TICK_TIMER: Duration = Duration::from_micros(1_000_000 / DT_TIMER_HZ);

        fn tick(&mut self) -> Result<(), Self::Error> {
            if self.do_error {
                return Err(MockError {});
            }

            self.tick += 1;
            self.ticks.push(0);
            Ok(())
        }

        fn tick_timer(&mut self) {
            self.tick_timer += 1;
            self.ticks.push(1);
        }
    }

    #[test]
    fn ticks_error() {
        let mut ticker = Ticker::default();
        let mut mock = MockTickable::new();

        mock.do_error = true;

        assert_eq!(
            ticker.tick(Duration::from_secs(1), &mut mock),
            Err(MockError {})
        );
    }

    #[test]
    fn ticks_count() {
        let mut ticker = Ticker::default();
        let mut mock = MockTickable::new();

        let _ = ticker.tick(Duration::from_secs(1), &mut mock);

        assert_eq!(mock.tick, DT_HZ as usize);
        assert_eq!(mock.tick_timer, DT_TIMER_HZ as usize);
    }

    #[test]
    fn ticks_interleaved() {
        let mut ticker = Ticker::default();
        let mut mock = MockTickable::new();

        let _ = ticker.tick(Duration::from_millis(16), &mut mock);

        assert_eq!(mock.tick, 8);
        assert_eq!(mock.tick_timer, 0);

        let _ = ticker.tick(Duration::from_millis(2), &mut mock);

        assert_eq!(mock.tick, 9);
        assert_eq!(mock.tick_timer, 1);

        let expected = [0, 0, 0, 0, 0, 0, 0, 0, 1, 0];

        assert_eq!(mock.ticks, expected);

        let _ = ticker.tick(Duration::from_millis(16), &mut mock);

        assert_eq!(mock.tick, 17);
        assert_eq!(mock.tick_timer, 2);

        let expected = [0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0];

        assert_eq!(mock.ticks, expected);

        let _ = ticker.tick(Duration::from_millis(16), &mut mock);

        assert_eq!(mock.tick, 25);
        assert_eq!(mock.tick_timer, 3);

        let expected = [
            0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0,
        ];

        assert_eq!(mock.ticks, expected);
    }
}
