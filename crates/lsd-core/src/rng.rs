/// The original executable's PsyQ 32-bit RNG and 15-bit result convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PsxRng {
    state: u32,
}

impl PsxRng {
    pub const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    pub const fn state(&self) -> u32 {
        self.state
    }

    pub fn reseed(&mut self, seed: u32) {
        self.state = seed;
    }

    pub fn next_u15(&mut self) -> u16 {
        self.state = self.state.wrapping_mul(0x41C6_4E6D).wrapping_add(0x3039);
        ((self.state >> 16) & 0x7FFF) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reseeding_restores_the_stream_including_zero() {
        let mut rng = PsxRng::new(0);
        let first = rng.next_u15();
        rng.next_u15();
        rng.reseed(0);
        assert_eq!(rng.next_u15(), first);
        assert_eq!(rng.state(), 12345);
    }

    #[test]
    fn cloned_state_continues_without_hidden_global_state() {
        let mut rng = PsxRng::new(u32::MAX);
        for _ in 0..15 {
            rng.next_u15();
        }
        let mut restored = rng;
        for _ in 0..64 {
            assert_eq!(rng.next_u15(), restored.next_u15());
        }
    }
}
