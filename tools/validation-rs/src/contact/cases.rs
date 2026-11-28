use super::oracle::{Input, Mu, Q, graph};

pub fn generate(mu: &Mu) -> Vec<Input> {
    let mut result = Vec::with_capacity(84886);
    for k in [0., 0.03, 0.1, 0.23] {
        let mut seq: Vec<_> = (-2000..=2000).map(|i| f64::from(i) / 1000.).collect();
        let knots: Vec<_> = Q.iter().zip(mu).map(|(q, m)| q + k * m).collect();
        for h in knots
            .iter()
            .copied()
            .chain([knots[0] - 1e-11, knots[0] + 1e-11])
        {
            seq.extend([h, -h]);
        }
        for &d in &seq {
            for prior in [-1, 0, 1] {
                result.push((d, k, prior));
            }
        }
        for ascending in [true, false] {
            // Stable sorting keeps the signed zeros in their original input order.
            let mut direction = seq.clone();
            direction.sort_by(|a, b| {
                if ascending {
                    a.partial_cmp(b).unwrap()
                } else {
                    b.partial_cmp(a).unwrap()
                }
            });
            let mut prior = 0;
            for d in direction {
                result.push((d, k, prior));
                prior = graph((d, k, prior), mu).unwrap().2;
            }
        }
    }
    for i in 0..9 {
        for width in [0., 5e-13, 1e-14] {
            let center = (Q[i + 1] - Q[i] - width) / (mu[i] - mu[i + 1]);
            for k in [center.next_down(), center, center.next_up()] {
                let (a, b) = (Q[i] + k * mu[i], Q[i + 1] + k * mu[i + 1]);
                for h in [a, b, (a + b) / 2.] {
                    for x in [h.next_down(), h, h.next_up()] {
                        for s in [-1, 1] {
                            for p in [-1, 0, 1] {
                                result.push((f64::from(s) * x, k, p));
                            }
                        }
                    }
                }
            }
        }
    }
    let h = 1.7431847738615466e-12f64;
    append_neighbors(&mut result, h, 4.5041501448100105e-13);
    let center = (Q[5] - Q[4]) / (mu[4] - mu[5]);
    let mut candidates = vec![center];
    let (mut lo, mut hi) = (center, center);
    for _ in 0..64 {
        lo = lo.next_down();
        hi = hi.next_up();
        candidates.extend([lo, hi]);
    }
    let tie = candidates
        .into_iter()
        .find(|k| Q[4] + k * mu[4] == Q[5] + k * mu[5])
        .expect("prepared coincident endpoints");
    append_neighbors(&mut result, Q[5] + tie * mu[5], tie);
    result
}
fn append_neighbors(result: &mut Vec<Input>, h: f64, k: f64) {
    for s in [-1, 1] {
        for x in [h.next_down(), h, h.next_up()] {
            result.push((f64::from(s) * x, k, s));
        }
    }
}
