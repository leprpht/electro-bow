use super::*;
use crate::{identity::stationary, pitch, recordings::scoring::Features};
use std::f64::consts::TAU;
type Orders = [f64; 12];
fn legacy_capture(x: &[f64], hz: f64) -> Result<(Orders, usize)> {
    let d = 3;
    let fa = 16000.;
    let host = 48000.;
    let c = (-TAU * 30. / host).exp();
    let fir = pitch::fir(d);
    let mut hp = Vec::new();
    let mut dec = Vec::new();
    let mut prev = 0.;
    for (i, &v) in x.iter().take(14400).enumerate() {
        let y = c * (hp.last().copied().unwrap_or(0.) + v - prev);
        prev = v;
        hp.push(y);
        if (i + 1) % d == 0 {
            dec.push(
                fir.iter()
                    .zip(hp.iter().rev())
                    .map(|(a, b)| a * b)
                    .sum::<f64>(),
            );
        }
    }
    let length = (4. * fa / hz + 0.5).floor() as usize;
    let hop = 80;
    let first = 400 + length;
    let a = -(-0.005_f64 / 0.04).exp_m1();
    let mut held = [0.; 12];
    let mut count = 0;
    for end in (first.div_ceil(hop) * hop..=dec.len()).step_by(hop) {
        if let Some(v) = capture::target(&dec[end - length..end], fa, hz) {
            count += 1;
            for (h, t) in held.iter_mut().zip(v) {
                *h += a * (t - *h);
            }
        }
    }
    Ok((held, count))
}
fn harmonics(x: &[f64], hz: f64) -> Orders {
    numerics::partials(x, 48000., hz, 12)
        .iter()
        .map(|z| z.norm())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}
fn unsafe_wet(raw: &[[f64; 30]], g: Orders) -> Result<Vec<f64>> {
    let mut bank = Bank::new(48000.)?;
    raw.iter()
        .map(|r| {
            let mut x = [0.; 17];
            x[0] = 196.;
            x[1] = r[25];
            x[5..].copy_from_slice(&g);
            Ok(x[1] + bank.apply(&x)?[0])
        })
        .collect()
}
fn carrier_g3(exe: &Path) -> Result<Vec<[f64; 30]>> {
    carrier::render(exe, &carrier::cases()?["steady_48000_196_R1.8_V0.1_3s"])
}
fn anchor(raw: &[[f64; 30]]) -> Result<f64> {
    stationary::Spectrum::new(
        &raw[raw.len() - RATE..]
            .iter()
            .map(|r| r[11])
            .collect::<Vec<_>>(),
        48000,
    )
    .peak
    .ok_or("missing M5 anchor".into())
}
pub(super) fn inversion(exe: &Path) -> Result<Value> {
    let raw = carrier_g3(exe)?;
    let f = anchor(&raw)?;
    let (g, count) = legacy_capture(&capture::stimulus(RATE, 196., "weak-H1"), 196.)?;
    let y = unsafe_wet(&raw, g.map(|v| 10f64.powf(v / 20.) - 1.))?;
    let i = raw.len() - RATE;
    let a = harmonics(&raw[i..].iter().map(|r| r[25]).collect::<Vec<_>>(), f);
    let b = harmonics(&y[i..], f);
    let ratios = [a, b].map(|h| 20. * (h[1] / h[0]).log10());
    let peak = stationary::Spectrum::new(&y[i..], 48000)
        .peak
        .ok_or("legacy inversion peak")?;
    ensure(
        count == 51
            && (peak - 391.84).abs() < 0.05
            && [ratios[0], ratios[1], g[0], g[1]]
                .iter()
                .zip([-1.32479, 9.23725, -4.99148, 4.99148])
                .all(|(v, w)| (v - w).abs() <= 0.002),
        "historical inversion",
    )?;
    Ok(json!({"updates":count,"gains":g,"peak":peak,"ratios":ratios}))
}
pub(super) fn run(exe: &Path, out: &Path, clips: &mut Vec<Clip>) -> Result<Value> {
    let raw = carrier_g3(exe)?;
    let f = anchor(&raw)?;
    let i = raw.len() - RATE;
    let neutral = raw[i..].iter().map(|r| r[25]).collect::<Vec<_>>();
    let mut takes = BTreeMap::new();
    let mut all = Vec::<(String, BTreeMap<&str, Orders>)>::new();
    let mut pairs = BTreeMap::new();
    for player in [1, 2] {
        for passage in [60, 132, 204] {
            let name = format!("techs-P{player}-{passage}");
            let (rate, samples) = recordings::read(
                &Path::new("tools/validation/fixtures/acoustic").join(format!("{name}.wav")),
            )?;
            ensure(rate == 48000, "source rate")?;
            let onset = samples
                .iter()
                .position(|v| v.abs() > 0.01)
                .ok_or("source onset")?;
            let x = &samples[onset..];
            let updates = capture::updates(x, RATE, 196.)?;
            let y = render(
                &raw,
                &vec![196.; raw.len()],
                Some(&updates),
                false,
                48000,
                [0.; 12],
            )?;
            let held = y.wet[i..].iter().map(|r| r[0]).collect::<Vec<_>>();
            let di = &x[3840..11040];
            let mut best = (0., 0.);
            for j in 0..241 {
                let hz = 193. + j as f64 * ((193.025_f64) - 193.);
                let a = numerics::partials(di, 48000., hz, 1)[0].norm();
                if a > best.0 {
                    best = (a, hz);
                }
            }
            if player == 1 && passage == 60 {
                let mut actions = crate::identity::clock::recorded_source(x)?;
                for action in &mut actions {
                    let t = action["tick"].as_u64().ok_or("source tick")? as usize;
                    let last = y
                        .log
                        .iter()
                        .rev()
                        .find(|r| r["end"].as_u64().is_some_and(|end| end + 17 <= t as u64));
                    action["applied_scale"] = y.scale[t].into();
                    action["veto"] = last.map_or(Value::Bool(true), |r| r["veto"].clone());
                    action["wet_FS"] = y.wet[t][0].into();
                    action["carrier_FS"] = raw[t][25].into();
                }
                evidence::atomic(&out.join("source-clock.json"), &json!(actions))?;
            }
            let h = harmonics(di, best.1);
            let level = rms(&x[2880..10560]);
            let normalized = x.iter().map(|v| v * 0.08 / level).collect::<Vec<_>>();
            let (historical, count) = legacy_capture(&normalized, 196.)?;
            let old = unsafe_wet(&raw, historical.map(|v| 10f64.powf(v / 20.) - 1.))?;
            let mut spectra: BTreeMap<_, _> = [
                ("di", h),
                ("safe", harmonics(&held, f)),
                ("unsafe", harmonics(&y.unsafe_off[i..], f)),
                ("neutral", harmonics(&neutral, f)),
                (
                    "requested",
                    updates
                        .last()
                        .ok_or("DI update")?
                        .1
                        .map(|g| 20. * (1. + g).log10()),
                ),
                (
                    "accepted",
                    y.final_coeff
                        .map(|g| 20. * (1. + g * y.scale.last().unwrap()).log10()),
                ),
                ("old_wet", harmonics(&old[i..], 196.)),
                ("old_di", harmonics(&normalized[3840..11040], 196.)),
            ]
            .into();
            for (other, peer) in &all {
                let mut row = json!({});
                for (key, b) in &spectra {
                    let a = peer[*key];
                    if matches!(*key, "requested" | "accepted") {
                        row[key] = rms(&std::array::from_fn::<_, 12, _>(|j| a[j] - b[j])).into();
                    } else {
                        let fa = Features::new(a);
                        let fb = Features::new(*b);
                        row[key] = rms(&std::array::from_fn::<_, 12, _>(|j| {
                            fa.vector[j] - fb.vector[j]
                        }))
                        .into();
                        row[format!("{key}_signed")] =
                            json!([fa.cues[0] - fb.cues[0], fa.cues[1] - fb.cues[1]]);
                    }
                }
                pairs.insert(format!("{other}/{name}"), row);
            }
            let mut features = BTreeMap::new();
            for k in ["di", "safe", "unsafe", "neutral"] {
                features.insert(k, Features::new(spectra[k]).value());
            }
            let mut depths = Vec::new();
            for j in 0..3 {
                let row = stationary::grade(&raw, &y.wet, rate, 196., j)?;
                ensure(
                    row["passes"]
                        .as_object()
                        .unwrap()
                        .values()
                        .all(|v| v != false),
                    "real source V17",
                )?;
                depths.push(row);
            }
            let norm = |x: &[f64]| x.iter().map(|v| v * v).sum::<f64>().sqrt();
            takes.insert(name.clone(),json!({"historical_updates":count,"source_features":features,"pitch":best.1,"onset":onset,"policy":policy(&y),"valid_publications":updates.len()-1,"bands_dB":([(0,2),(2,4),(4,8),(8,12)].map(|(a,b)|20.*(norm(&h[a..b])/norm(&h)).max(1e-12).log10())),"depths":depths}));
            all.push((name.clone(), std::mem::take(&mut spectra)));
            if passage == 60 {
                clips.push((format!("{name}-DI"), x[..19200].to_vec(), 2880..10560));
                clips.push((format!("{name}-held"), held, 0..RATE));
            }
        }
    }
    clips.push(("neutral-held".into(), neutral, 0..RATE));
    Ok(json!({"takes":takes,"pairs":pairs}))
}
