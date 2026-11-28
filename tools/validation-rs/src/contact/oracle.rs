//! Independent intersection of the stored graph, including its descending sections.
use crate::{Result, ensure};
use num_rational::BigRational;
use num_traits::ToPrimitive;

pub const Q: [f64; 10] = [0., 0.005, 0.01, 0.02, 0.04, 0.08, 0.16, 0.32, 0.64, 1.28];
pub type Mu = [f64; 10];
pub type Input = (f64, f64, i8);
pub type Answer = (f64, f64, i8);

pub fn ulp(x: f64) -> f64 {
    let a = x.abs();
    if a == f64::MAX {
        a - a.next_down()
    } else {
        a.next_up() - a
    }
}
fn rational(x: f64) -> BigRational {
    BigRational::from_float(x).expect("finite graph endpoint")
}
pub fn intersections(h: f64, k: f64, mu: &Mu) -> Vec<(f64, usize, bool)> {
    let height: Vec<_> = Q.iter().zip(mu).map(|(q, m)| q + k * m).collect();
    let mut roots = Vec::new();
    for (i, pair) in height.windows(2).enumerate() {
        let (lo, hi) = (pair[0], pair[1]);
        if h < lo.min(hi) || h > lo.max(hi) {
            continue;
        }
        if lo == hi {
            roots.extend([(Q[i], i, true), (Q[i + 1], i, true)]);
        } else {
            let (a, b, x) = (rational(lo), rational(hi), rational(h));
            let q = ((&b - &x) * rational(Q[i]) + (&x - &a) * rational(Q[i + 1])) / (b - a);
            roots.push((q.to_f64().expect("finite root"), i, lo < hi));
        }
    }
    if h >= height[9] {
        roots.push((if h == height[9] { Q[9] } else { h - k * mu[9] }, 9, true));
    }
    roots
}
pub fn graph((d, k, prior): Input, mu: &Mu) -> Option<Answer> {
    if !d.is_finite() || !k.is_finite() || k < 0. {
        return None;
    }
    if k == 0. {
        return Some((0., d, 0));
    }
    let limit = k * mu[0];
    if !limit.is_finite() {
        return None;
    }
    let h = d.abs();
    let sign = if d > 0. { 1 } else { -1 };
    let mut answer = (d, 0., 0);
    if d != 0. && (prior == sign || h > limit) {
        // Retain the first equal root, as in Python max(key=...).
        let mut best = None;
        for (q, i, stable) in intersections(h, k, mu) {
            if stable && best.is_none_or(|(old, _)| q > old) {
                best = Some((q, i));
            }
        }
        if let Some((q, i)) = best {
            let left = Q[i] + k * mu[i];
            let rounded = if (q - 1e-12).abs() > 4. * ulp(q) || i == 9 {
                q
            } else {
                let right = Q[i + 1] + k * mu[i + 1];
                if h == right {
                    Q[i + 1]
                } else {
                    Q[i] + (Q[i + 1] - Q[i]) * ((h - left) / (right - left))
                }
            };
            if rounded > 1e-12 {
                let slip = f64::from(sign) * q;
                answer = (d - slip, slip, sign);
            }
        } else if h <= limit + 1e-12 {
            answer.0 = h.min(limit).copysign(d);
        } else {
            return None;
        }
    }
    (2. * answer.0).is_finite().then_some(answer)
}
pub fn physics((d, k, _): Input, (j, r, mode): Answer, mu: &Mu, exact: bool) -> Result<()> {
    let a = 0.13;
    let b = 0.1 - a - d;
    let v = a + b + j;
    let force = 2. * j;
    let power = (a + j).powi(2) + (b + j).powi(2) - a * a - b * b;
    ensure(
        (power - force * v).abs() < 1e-10 && force * r >= -1e-12,
        "flux/dissipation",
    )?;
    ensure(
        ((0.1 - v) - r).abs() < 1e-10
            && (d - r - j).abs() < 1e-10
            && force.abs() <= 2. * k * mu[0] + 2e-10,
        "contact residual/force",
    )?;
    if k == 0. {
        return ensure((j, r, mode) == (0., d, 0), "zero pressure");
    }
    if mode == 0 {
        return ensure(r.abs() < 1e-10 && (d - j).abs() < 1e-10, "static residual");
    }
    let q = r.abs();
    let i = Q.partition_point(|v| *v <= q) - 1;
    let mut value = mu[i];
    if i + 1 < Q.len() {
        value += (mu[i + 1] - mu[i]) * (q - Q[i]) / (Q[i + 1] - Q[i]);
    }
    ensure(
        q > 1e-12 || (exact && (q - 1e-12).abs() <= 4. * ulp(q)),
        "slip floor",
    )?;
    ensure(
        f64::from(mode) == r.signum()
            && r * d > 0.
            && (q + k * value - d.abs()).abs() < 1e-10
            && (j - (k * value).copysign(r)).abs() < 1e-10,
        "sliding friction law",
    )
}
pub fn compare(d: f64, (j, r, m): Answer, (ej, er, em): Answer) -> Result<()> {
    ensure(m == em, "branch mode")?;
    let tol = 4. * ulp(er);
    ensure((r - er).abs() <= tol, "slip ULP")?;
    ensure(
        (j - ej).abs() < 1e-10 && ((d - j) - er).abs() <= tol + ulp(j) + ulp(d - j),
        "reconstructed impulse/slip",
    )
}
