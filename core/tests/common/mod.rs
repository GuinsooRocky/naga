#![allow(dead_code)]

/// java.util.Random 的同款算法：同一个种子造出的测试信号和原来 Kotlin 版测试完全一样，
/// 两边的误差数字才能直接对比。
pub struct JavaRandom {
    seed: u64,
    spare_gaussian: Option<f64>,
}

const MULTIPLIER: u64 = 0x5DEECE66D;
const MASK: u64 = (1 << 48) - 1;

impl JavaRandom {
    pub fn new(seed: i64) -> Self {
        Self { seed: (seed as u64 ^ MULTIPLIER) & MASK, spare_gaussian: None }
    }

    fn next(&mut self, bits: u32) -> i64 {
        self.seed = self.seed.wrapping_mul(MULTIPLIER).wrapping_add(0xB) & MASK;
        (self.seed >> (48 - bits)) as i64
    }

    pub fn next_double(&mut self) -> f64 {
        ((self.next(26) << 27) + self.next(27)) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    pub fn next_gaussian(&mut self) -> f64 {
        if let Some(g) = self.spare_gaussian.take() {
            return g;
        }
        loop {
            let v1 = 2.0 * self.next_double() - 1.0;
            let v2 = 2.0 * self.next_double() - 1.0;
            let s = v1 * v1 + v2 * v2;
            if s < 1.0 && s != 0.0 {
                let multiplier = (-2.0 * s.ln() / s).sqrt();
                self.spare_gaussian = Some(v2 * multiplier);
                return v1 * multiplier;
            }
        }
    }
}
