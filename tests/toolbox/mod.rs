use rand::thread_rng;
use rand::distributions::{Range, Sample};

const PRIVATE_PORTS_MIN: u16 = 49_152;
const PRIVATE_PORTS_MAX: u16 = 65_535;

pub fn random_port() -> u16 {
    Range::new(PRIVATE_PORTS_MIN, PRIVATE_PORTS_MAX).sample(&mut thread_rng())
}
