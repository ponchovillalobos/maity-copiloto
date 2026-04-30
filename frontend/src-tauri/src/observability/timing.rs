//! Helper para medir latencias de etapas del pipeline.

use std::time::Instant;

/// Timer simple. `start()` captura ahora; `elapsed_ms()` devuelve ms transcurridos.
pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_timer_basic() {
        let t = Timer::start();
        sleep(Duration::from_millis(20));
        let e = t.elapsed_ms();
        assert!(e >= 18 && e < 200, "elapsed_ms={}", e);
    }
}
