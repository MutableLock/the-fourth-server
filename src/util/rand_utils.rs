use rand::distr::{Distribution, Uniform};
use rand::rng;


pub fn generate_random_u8_vec(len: usize) -> Vec<u8> {
    let mut rng = rng();
    let dist = Uniform::new(0, u8::MAX);
    (0..len).map(|_| dist.unwrap().sample(&mut rng)).collect()
}