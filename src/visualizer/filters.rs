pub struct HighPassIIR {
    alpha: f32,
    delta: f32,
    prev_in: f32,
    prev_out: f32
}

impl HighPassIIR {
    pub fn new(sample_rate: f32, cutoff_frequency: f32) -> Self {
        let period = 1.0 / sample_rate;
        let tc = 1.0 / cutoff_frequency;

        Self {
            alpha: tc / (tc + period),
            delta: 0.0,
            prev_in: 0.0,
            prev_out: 0.0
        }
    }

    pub fn consume(&mut self, input: f32) {
        self.prev_out = self.output();
        self.delta = input - self.prev_in;
        self.prev_in = input;
    }

    pub fn output(&self) -> f32 {
        self.alpha * self.prev_out + self.alpha * self.delta
    }
}
