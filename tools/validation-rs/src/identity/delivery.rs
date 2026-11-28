//! The candidate's final, pre-linked output boundary. Only a checked live
//! sample can become the preceding finite pair.
#[derive(Default, Clone, Copy)]
pub struct Sample {
    pub normalized: f64,
    pub gain: f64,
    pub signed: f64,
    pub energy: f64,
    pub product: f64,
    pub character: f64,
    pub direct: f64,
    pub mix: f64,
    pub output: f32,
}
impl Sample {
    pub fn live(
        normalized: f64,
        gain: f64,
        energy: f64,
        character: f64,
        direct: f64,
    ) -> Result<Self, i32> {
        let signed = gain * normalized;
        let product = energy * signed;
        if !(0.08..=1.4).contains(&gain.abs()) || !signed.is_finite() || !product.is_finite() {
            return Err(22);
        }
        if !(0. ..=1.).contains(&energy) {
            return Err(10);
        }
        let mut sample = Self {
            normalized,
            gain,
            signed,
            energy,
            product,
            character,
            direct,
            ..Self::default()
        };
        sample.convert()?;
        Ok(sample)
    }
    fn convert(&mut self) -> Result<(), i32> {
        self.mix =
            (1. - self.character * self.energy) * self.direct + self.character * self.product;
        self.output = self.mix as f32;
        if self.output.is_finite() {
            Ok(())
        } else {
            Err(10)
        }
    }
    pub fn inactive(pair: [f64; 2], gain: f64, character: f64, direct: f64) -> Result<Self, i32> {
        let mut sample = Self {
            energy: pair[0],
            product: pair[1],
            gain,
            character,
            direct,
            ..Self::default()
        };
        sample.convert()?;
        Ok(sample)
    }
}
