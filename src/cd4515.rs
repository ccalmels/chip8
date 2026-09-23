pub struct Cd4515 {
    keys: u16,
    is_waiting: bool,
    last_key: Option<u8>,
}

impl Cd4515 {
    pub fn new() -> Self {
        Cd4515 {
            keys: 0,
            is_waiting: false,
            last_key: None,
        }
    }

    pub fn key_pressed(&self, key: u8) -> bool {
        self.keys & (1 << key) != 0x0
    }

    pub fn wait_key(&mut self) -> Option<u8> {
        if !self.is_waiting {
            self.is_waiting = true;
            self.last_key = None;
            None
        } else if let Some(key) = self.last_key {
            self.is_waiting = false;
            Some(key)
        } else {
            None
        }
    }

    pub fn key_event(&mut self, key: u8, is_pressed: bool) {
        if is_pressed {
            self.keys |= 1 << key;
        } else {
            self.keys ^= 1 << key;

            if self.is_waiting && self.last_key.is_none() {
                self.last_key = Some(key);
            }
        }
    }
}

impl Default for Cd4515 {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod cd4515_tests {
    use super::*;

    #[test]
    fn pressed() {
        let mut cd4515 = Cd4515::default();

        assert!(!cd4515.key_pressed(2));
        assert!(!cd4515.key_pressed(0xf));

        cd4515.key_event(2, true);

        assert!(cd4515.key_pressed(2));
        assert!(!cd4515.key_pressed(0xf));

        cd4515.key_event(0xf, true);

        assert!(cd4515.key_pressed(2));
        assert!(cd4515.key_pressed(0xf));

        cd4515.key_event(2, false);

        assert!(!cd4515.key_pressed(2));
        assert!(cd4515.key_pressed(0xf));

        cd4515.key_event(0xf, false);

        assert!(!cd4515.key_pressed(2));
        assert!(!cd4515.key_pressed(0xf));
    }

    #[test]
    fn wait_key() {
        let mut cd4515 = Cd4515::default();

        assert_eq!(cd4515.wait_key(), None);

        cd4515.key_event(3, true);

        assert_eq!(cd4515.wait_key(), None);

        cd4515.key_event(3, false);

        assert_eq!(cd4515.wait_key(), Some(3));

        assert_eq!(cd4515.wait_key(), None);
    }

    #[test]
    fn wait_key_first_released() {
        let mut cd4515 = Cd4515::default();

        cd4515.key_event(3, true);
        cd4515.key_event(5, true);
        cd4515.key_event(0xa, true);

        assert_eq!(cd4515.wait_key(), None);

        cd4515.key_event(0xa, false);
        cd4515.key_event(3, false);
        cd4515.key_event(5, false);

        assert_eq!(cd4515.wait_key(), Some(0xa));

        assert_eq!(cd4515.wait_key(), None);
    }

    #[test]
    fn wait_key_already_pressed() {
        let mut cd4515 = Cd4515::default();

        cd4515.key_event(5, true);

        assert_eq!(cd4515.wait_key(), None);

        cd4515.key_event(3, true);

        assert_eq!(cd4515.wait_key(), None);

        cd4515.key_event(5, false);

        assert_eq!(cd4515.wait_key(), Some(5));

        assert_eq!(cd4515.wait_key(), None);
    }
}
