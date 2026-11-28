//! Stage 08.3 offline candidate. Packed sufficient statistics and in-place
//! triangular substitutions; the rectangular QR and reference bank live elsewhere.
use rustfft::num_complex::Complex64;
use std::f64::consts::{SQRT_2, TAU};

type Checked<T> = Result<T, i32>;
fn finite(x: f64, site: i32) -> Checked<f64> {
    if x.is_finite() { Ok(x) } else { Err(site) }
}

#[repr(C)]
#[derive(Default)]
pub struct Slew {
    pub value: f64,
    origin: f64,
    destination: f64,
    elapsed: u32,
}
impl Slew {
    pub fn advance(&mut self, dest: f64, n: u32) -> f64 {
        if dest != self.destination {
            self.origin = self.value;
            self.destination = dest;
            self.elapsed = 0;
        }
        if self.value != dest {
            self.elapsed += 1;
            let distance = f64::from(self.elapsed) / f64::from(n);
            let delta = dest - self.origin;
            self.value = if distance >= delta.abs() {
                dest
            } else {
                self.origin + distance.copysign(delta)
            };
        }
        self.value
    }
}

// Row-major lower triangle, then thirteen contiguous six-word projections.
fn tri(i: usize, j: usize) -> usize {
    let hi = i.max(j);
    hi * (hi + 1) / 2 + i.min(j)
}
type Words = [f64; 123];
const U: usize = 21;
const QD: usize = 99;
const YY: usize = 111;
const MM: usize = 115;
const PAIR: usize = 116;
const DIFF: usize = 119;

struct Fit<'a> {
    words: &'a Words,
    n: u32,
}
impl Fit<'_> {
    fn positive(&self, x: f64, magnitude: f64) -> Checked<f64> {
        let tol = finite(64. * f64::from(self.n) * f64::EPSILON * magnitude, 8)?;
        if finite(x, 8)? < -tol {
            Err(8)
        } else {
            Ok(x.max(0.))
        }
    }
    fn energy(&self, a: &[f64], lo: usize, hi: usize) -> Checked<f64> {
        let (mut sum, mut magnitude) = (0., 0.);
        for i in lo..hi {
            for j in lo..hi {
                let term = a[i] * self.words[tri(i, j)] * a[j];
                sum += term;
                magnitude += term.abs();
            }
        }
        self.positive(sum, magnitude)
    }
    fn power(&self, offset: usize, w: f64) -> Checked<f64> {
        let [q, cross, d]: [f64; 3] = self.words[offset..offset + 3].try_into().unwrap();
        let cross = 2. * w * cross;
        let d = (w * w) * d;
        self.positive((q + cross) + d, q.abs() + cross.abs() + d.abs())
    }
    fn passes(&self, a: &[f64], p: f64, neutral: f64, recovery: bool) -> Checked<bool> {
        let fit = self.energy(a, 0, 6)?;
        let rest = self.positive(p - fit, p.abs() + fit)?.sqrt();
        let h1 = self.energy(a, 1, 3)?.sqrt();
        let h2 = self.energy(a, 3, 5)?.sqrt();
        let factor = 10f64.powf(if recovery { 2.85 } else { 3. } / 10.);
        let margin = 10f64.powf(if recovery { 0.30 } else { 0.05 } / 20.);
        let m5 = self.words[MM];
        Ok(m5 > 1e-24
            && neutral > 1e-24
            && h1 >= margin * h2.max(SQRT_2 * a[5].abs()).max(rest) + 1e-12
            && p >= m5 / factor
            && p <= finite(m5 * factor, 4)?
            && p >= neutral / factor
            && p <= finite(neutral * factor, 4)?)
    }
    fn candidate(&self, w: f64) -> Checked<bool> {
        let mut ok = true;
        for j in 0..3 {
            let mut a = [0.; 6];
            let base = U + 18 * j;
            for (i, v) in a.iter_mut().enumerate() {
                *v = finite(self.words[base + i] + w * self.words[base + 6 + i], 4)?;
            }
            ok &= self.passes(&a, self.power(QD + 3 * j, w)?, self.words[QD + 3 * j], true)?;
        }
        Ok(ok && self.power(PAIR, w)? <= 10f64.powf(-2.85) * self.power(QD, w)?)
    }
}
#[repr(C)]
pub struct Candidate {
    blocks: [Words; 2],
    first: [[Complex64; 12]; 4],
    second: [[Complex64; 12]; 4],
    poles: [Complex64; 12],
    taper: [f64; 12],
    fs: f64,
    last_pitch: f64,
    radius: f64,
    pub phase: f64,
    slew: Slew,
    target: f64,
    selected: f64,
    current: usize,
    count: u32,
    length: u32,
    retired_n: u32,
    trial: i32,
    best: i32,
    clean: u32,
    observed: bool,
    valid: bool,
    bridge: bool,
    available: bool,
    live: bool,
    veto: bool,
    pending: bool,
}
impl Candidate {
    pub fn new(fs: f64) -> Self {
        Self {
            fs,
            blocks: [[0.; 123]; 2],
            first: [[Complex64::ZERO; 12]; 4],
            second: [[Complex64::ZERO; 12]; 4],
            poles: [Complex64::ZERO; 12],
            taper: [0.; 12],
            last_pitch: 0.,
            radius: 0.,
            phase: 0.,
            slew: Slew::default(),
            target: 0.,
            selected: 0.,
            current: 0,
            count: 0,
            length: 0,
            retired_n: 0,
            trial: -1,
            best: -1,
            clean: 0,
            observed: true,
            valid: false,
            bridge: false,
            available: false,
            live: false,
            veto: true,
            pending: false,
        }
    }
    pub fn stats(&self, retired: bool) -> Words {
        self.blocks[self.current ^ usize::from(retired)]
    }
    fn retire(&mut self, eligible: bool, probe: i32) -> Checked<()> {
        let s = &mut self.blocks[self.current];
        self.retired_n = self.length;
        let inv = if probe == 6 {
            f64::MAX
        } else {
            1. / f64::from(self.length)
        };
        for v in s.iter_mut() {
            *v = finite(*v * inv, 6)?;
        }
        if probe == 7 {
            s[0] = f64::NAN;
        }
        self.valid = true;
        for i in 0..6 {
            let r = (0..6)
                .filter(|&j| j != i)
                .map(|j| s[tri(i, j)].abs())
                .sum::<f64>();
            self.valid &=
                finite(s[tri(i, i)] - r, 7)? >= 0.25 && finite(s[tri(i, i)] + r, 7)? <= 2.;
        }
        self.live = false;
        self.bridge = false;
        if self.valid {
            let mut factor = [0.; 21];
            for i in 0..6 {
                for j in 0..=i {
                    let mut v = s[tri(i, j)];
                    for k in 0..j {
                        v -= factor[tri(i, k)] * factor[tri(j, k)];
                    }
                    factor[tri(i, j)] = finite(
                        if i == j {
                            v.sqrt()
                        } else {
                            v / factor[tri(j, j)]
                        },
                        7,
                    )?;
                }
            }
            for a in s[U..QD].as_chunks_mut::<6>().0 {
                for i in 0..6 {
                    for j in 0..i {
                        a[i] -= factor[tri(i, j)] * a[j];
                    }
                    a[i] = finite(a[i] / factor[tri(i, i)], 7)?;
                }
                for i in (0..6).rev() {
                    for j in i + 1..6 {
                        a[i] -= factor[tri(j, i)] * a[j];
                    }
                    a[i] = finite(a[i] / factor[tri(i, i)], 7)?;
                }
            }
            if probe == 8 {
                s[YY + 3] = -1.;
            }
            let fit = Fit {
                words: s,
                n: self.length,
            };
            self.bridge = fit.passes(&s[U + 72..QD], s[MM], s[MM], false)?;
            self.live = true;
            for j in 0..4 {
                self.live &= fit.passes(&s[U + 18 * j + 12..], s[YY + j], s[QD + 3 * j], false)?;
            }
            self.live &= s[DIFF] <= 10f64.powf(-2.8) * s[YY];
        }
        self.available = self.valid && self.bridge && eligible && self.observed;
        self.phase %= TAU;
        self.observed = true;
        self.count = 0;
        self.trial = 0;
        self.best = -1;
        self.current ^= 1;
        self.blocks[self.current].fill(0.);
        Ok(())
    }
    fn basis(&mut self, f: f64, probe: i32) -> Checked<[f64; 6]> {
        if probe == 1 {
            self.phase = f64::NAN;
        }
        // The previous close reduced phase modulo tau. Each committed step
        // is bounded by the maximum supported pitch at this sample rate.
        let limit = TAU + (f64::from(self.count) + 1.) * TAU * 1320. / self.fs;
        if !(0. ..=limit).contains(&self.phase) {
            return Err(1);
        }
        let inc = if probe == 11 {
            f64::INFINITY
        } else {
            TAU * f / self.fs
        };
        self.phase = finite(self.phase + inc, 1)?;
        let doubled = finite(2. * self.phase, 1)?;
        if doubled > 2. * (limit + TAU * 1320. / self.fs) {
            return Err(1);
        }
        let b = [
            1.,
            self.phase.cos(),
            self.phase.sin(),
            doubled.cos(),
            doubled.sin(),
            if self.count.is_multiple_of(2) {
                1.
            } else {
                -1.
            },
        ];
        if b.iter().all(|x| (-1. ..=1.).contains(x)) {
            Ok(b)
        } else {
            Err(1)
        }
    }
    pub fn tick(&mut self, frame: &[f64; 20], out: &mut [f64; 20], probe: i32) -> Checked<()> {
        let (f, m) = (frame[0], frame[1]);
        let eligible = frame[19] != 0.;
        if !eligible {
            self.available = false;
            self.pending = false;
            self.target = 0.;
            self.selected = 0.;
            self.clean = 0;
            self.veto = true;
        }
        if self.pending {
            self.target = self.selected;
            self.pending = false;
        }
        if !(65. ..=1320.).contains(&f) {
            return Err(1);
        }
        if self.count == 0 {
            self.length = 2 * (4. * self.fs / f + 0.5).floor() as u32;
        }
        if f != self.last_pitch {
            self.radius = (-TAU * 0.22 * f / self.fs).exp();
            let fa = self.fs / (self.fs / 16000.).floor();
            for k in 0..12 {
                let order = (k + 1) as f64;
                self.poles[k] = Complex64::from_polar(self.radius, TAU * order * f / self.fs);
                self.taper[k] = ((0.28 * fa - order * f) / (0.03 * fa)).clamp(0., 1.);
            }
            self.last_pitch = f;
        }
        if probe == 3 {
            self.second[3][0].re = f64::INFINITY;
        }
        if probe == 13 {
            self.first[3][0] = Complex64::new(f64::MAX, f64::MAX);
            self.second[3][0] = self.first[3][0];
        }
        let mut d = [0.; 4];
        for j in 0..4 {
            for k in 0..12 {
                let z = self.poles[k] * self.first[j][k] + (1. - self.radius) * frame[2 + j];
                let zz = self.poles[k] * self.second[j][k] + (1. - self.radius) * z;
                if !z.is_finite() || !zz.is_finite() {
                    return Err(3);
                }
                self.first[j][k] = z;
                self.second[j][k] = zz;
                let coeff = finite(
                    if probe == 2 && k == 0 {
                        f64::NAN
                    } else {
                        frame[6 + k]
                    },
                    2,
                )?;
                d[j] = finite(d[j] + coeff * self.taper[k] * 2. * zz.re, 3)?;
            }
        }
        let b = self.basis(f, probe)?;
        out[9] = f64::from(self.trial);
        out[19] = -1.;
        if self.trial >= 0 {
            let s = &mut self.blocks[1 - self.current];
            if probe == 4 {
                s[QD + 1] = f64::MAX;
            }
            if probe == 14 {
                s[U] = f64::NAN;
            }
            let index = 16 - self.trial;
            let ok = self.valid
                && Fit {
                    words: s,
                    n: self.retired_n,
                }
                .candidate(f64::from(index) / 16.)?;
            out[19] = f64::from(ok);
            if ok && self.best < 0 {
                self.best = index;
            }
            self.trial += 1;
            if self.trial == 17 {
                self.clean = if ok && self.available && self.live {
                    (self.clean + 1).min(2)
                } else {
                    0
                };
                self.veto = self.clean < 2;
                self.selected = if self.veto {
                    0.
                } else {
                    f64::from(self.best) / 16.
                };
                self.pending = true;
                self.trial = -1;
            }
        }
        let n = (0.005 * self.fs).round() as u32;
        let scale = self.slew.advance(self.target, n);
        let mut y = [0.; 4];
        for j in 0..4 {
            let q = frame[2 + j];
            y[j] = finite(
                if probe == 9 && j == 3 {
                    f64::MAX + f64::MAX
                } else if scale == 0. {
                    q
                } else {
                    q + scale * d[j]
                },
                9,
            )?;
        }
        out[..4].copy_from_slice(&d);
        out[4..8].copy_from_slice(&y);
        let s = &mut self.blocks[self.current];
        let mut add = |k: usize, a: f64, v: f64| -> Checked<()> {
            s[k] = finite(s[k] + a * v, 5)?;
            Ok(())
        };
        for i in 0..6 {
            for k in 0..=i {
                add(tri(i, k), b[i], b[k])?;
            }
            for j in 0..4 {
                let base = U + 18 * j + i;
                add(base, b[i], frame[2 + j])?;
                add(base + 6, b[i], d[j])?;
                add(base + 12, b[i], y[j])?;
            }
            add(U + 72 + i, b[i], m)?;
        }
        for j in 0..4 {
            let q = frame[2 + j];
            add(QD + 3 * j, q, q)?;
            add(QD + 3 * j + 1, q, d[j])?;
            add(QD + 3 * j + 2, d[j], d[j])?;
            add(YY + j, y[j], y[j])?;
        }
        add(MM, m, if probe == 5 { f64::INFINITY } else { m })?;
        let x = frame[3] - frame[2];
        let v = d[1] - d[0];
        let diff = y[1] - y[0];
        let pre = frame[5] + self.target * d[3];
        add(PAIR, x, x)?;
        add(PAIR + 1, x, v)?;
        add(PAIR + 2, v, v)?;
        add(DIFF, diff, diff)?;
        add(120, pre, pre)?;
        add(121, frame[5], y[3])?;
        add(122, d[3], y[3])?;
        self.observed &= eligible && frame[18] != 0.;
        self.count += 1;
        let closed = self.count == self.length;
        if closed {
            self.retire(eligible, probe)?;
        }
        let terminal = if probe == 10 { f64::MAX } else { y[3] };
        if terminal.abs() > f64::from(f32::MAX) {
            return Err(10);
        }
        out[8] = scale;
        out[10..19].copy_from_slice(&[
            f64::from(closed),
            f64::from(self.live),
            f64::from(self.valid),
            f64::from(self.bridge),
            f64::from(self.available),
            f64::from(self.veto),
            self.selected,
            self.phase,
            self.target,
        ]);
        Ok(())
    }
}

// Qualify only the two permitted changed columns (permission and its four wet paths).
// The historical caller separately authenticates all of its frozen hashes.
pub(super) struct Differential {
    epoch: Candidate,
    ticks: u64,
    origin: f64,
    previous: f64,
    target_tick: u64,
    target: f64,
    pub changed_wet: u64,
    pub changed_audio: u64,
}
impl Differential {
    pub fn new(fs: f64) -> Self {
        Self {
            epoch: Candidate::new(fs),
            ticks: 0,
            origin: 0.,
            previous: 0.,
            target_tick: 0,
            target: 0.,
            changed_wet: 0,
            changed_audio: 0,
        }
    }
    pub fn check(&mut self, input: &[f64; 20], historical: &[f64; 20]) -> crate::Result<[f64; 20]> {
        let mut actual = [0.; 20];
        self.epoch
            .tick(input, &mut actual, 0)
            .map_err(|site| format!("corrected epoch fault {site}"))?;
        for j in (0..4).chain(9..20) {
            crate::ensure(
                actual[j].to_bits() == historical[j].to_bits(),
                "unexpected bank, phase, trial or monitor divergence",
            )?;
        }
        let target = historical[18];
        if target != self.target {
            self.target_tick = self.ticks;
            self.origin = self.previous;
            self.target = target;
        }
        let travelled =
            (self.ticks - self.target_tick + 1) as f64 / (0.005 * self.epoch.fs).round();
        let expected = if target > self.origin {
            (self.origin + travelled).min(target)
        } else {
            (self.origin - travelled).max(target)
        };
        crate::ensure(
            actual[8] == expected,
            "corrected permission from qualified target schedule",
        )?;
        self.previous = expected;
        self.ticks += 1;
        for j in 0..4 {
            let q = input[2 + j];
            let wet = if expected == 0. {
                q
            } else {
                q + expected * historical[j]
            };
            crate::ensure(
                actual[4 + j].to_bits() == wet.to_bits(),
                "corrected normalized wet",
            )?;
            self.changed_wet += u64::from(actual[4 + j].to_bits() != historical[4 + j].to_bits());
            self.changed_audio +=
                u64::from((actual[4 + j] as f32).to_bits() != (historical[4 + j] as f32).to_bits());
        }
        Ok(actual)
    }
}

// Concrete offsets in the corrected epoch, including its internal padding.
pub(super) fn field_offsets() -> serde_json::Value {
    use std::mem::{align_of, offset_of, size_of};
    let mut fields = serde_json::Map::new();
    macro_rules! field {
        ($($name:ident:$ty:ty),*) => {$(fields.insert(stringify!($name).into(),serde_json::json!({"offset":offset_of!(Candidate,$name),"bytes":size_of::<$ty>(),"alignment":align_of::<$ty>(),"type":stringify!($ty)}));)*};
    }
    field!(blocks:[Words;2],first:[[Complex64;12];4],second:[[Complex64;12];4],poles:[Complex64;12],taper:[f64;12],fs:f64,last_pitch:f64,radius:f64,phase:f64,slew:Slew,target:f64,selected:f64,current:usize,count:u32,length:u32,retired_n:u32,trial:i32,best:i32,clean:u32,observed:bool,valid:bool,bridge:bool,available:bool,live:bool,veto:bool,pending:bool);
    serde_json::json!({"fields":fields,"slew_offsets":{"value":offset_of!(Slew,value),"origin":offset_of!(Slew,origin),"destination":offset_of!(Slew,destination),"elapsed_u32":offset_of!(Slew,elapsed)},"bytes":size_of::<Candidate>(),"alignment":align_of::<Candidate>()})
}
const _: () =
    assert!(std::mem::size_of::<Words>() == 984 && std::mem::align_of::<Candidate>() == 8);
