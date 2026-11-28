use super::oracle;
use crate::{
    Result, ensure, evidence,
    observation::{Case, controls, float_wav},
    replay,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

pub fn cases() -> Result<BTreeMap<String, Case>> {
    let mut cases: BTreeMap<String, Case> =
        serde_json::from_str(include_str!("../../contracts/physical-cases.json"))?;
    cases.extend(serde_json::from_str::<BTreeMap<String, Case>>(
        include_str!("../../contracts/extra-physical.json"),
    )?);
    ensure(cases.len() == 156, "full carrier inventory")?;
    Ok(cases)
}
pub fn render(exe: &Path, c: &Case) -> Result<Vec<[f64; 30]>> {
    let n = c.successful_ticks;
    let input = controls(c);
    ensure(
        format!("{:x}", Sha256::digest(&input)) == c.controls_sha256,
        "electrical control schedule bytes",
    )?;
    scheduled(exe, c.rate, n, &input)
}
pub fn scheduled(exe: &Path, rate: u32, n: usize, input: &[u8]) -> Result<Vec<[f64; 30]>> {
    let output = replay::stream(exe, &["--o2-guide".into(), rate.to_string()], input)?;
    let counts: Vec<usize> = std::str::from_utf8(&output.stderr)?
        .split_whitespace()
        .map(str::parse)
        .collect::<std::result::Result<_, _>>()?;
    ensure(
        counts == [n, 1, 0, 1, n, n] && output.stdout.len() == 240 * n,
        "electrical epoch counters and frame count",
    )?;
    Ok(output
        .stdout
        .as_chunks::<240>()
        .0
        .iter()
        .map(|row| std::array::from_fn(|i| f64::from_le_bytes(row.as_chunks::<8>().0[i])))
        .collect())
}
pub fn identities() -> BTreeMap<&'static str, [&'static str; 3]> {
    include_str!("../../../validation/fixtures/o2_replay_sha256.tsv")
        .lines()
        .skip(1)
        .map(|l| {
            let v: Vec<_> = l.split('\t').collect();
            (v[0], [v[1], v[2], v[3]])
        })
        .collect()
}
pub fn run(out: &Path) -> Result<Value> {
    let exe = replay::build(out, None)?;
    let producer = evidence::digest(&exe)?;
    let identities = identities();
    let mut result = BTreeMap::new();
    let traces = out.join("traces");
    let audio = out.join("audio");
    fs::create_dir(&traces)?;
    fs::create_dir(&audio)?;
    for (name, c) in cases()? {
        let expected = identities
            .get(name.as_str())
            .ok_or("missing carrier identity")?;
        let rows = render(&exe, &c)?;
        let mut original = Sha256::new();
        let mut observed = Sha256::new();
        let mut trace = Sha256::new();
        let mut taps: [Sha256; 8] = std::array::from_fn(|_| Sha256::new());
        let mut raw = Vec::new();
        for row in &rows {
            let bytes = row.map(f64::to_le_bytes);
            let bytes = bytes.as_flattened();
            original.update(&bytes[..96]);
            observed.update(&bytes[..176]);
            trace.update(bytes);
            if expected[2] != "-" {
                raw.extend(bytes);
                for (h, j) in taps.iter_mut().zip([3, 11, 21, 22, 24, 25, 26, 27]) {
                    h.update(row[j].to_le_bytes());
                }
            }
        }
        let actual = [
            format!("{:x}", original.finalize()),
            format!("{:x}", observed.finalize()),
            format!("{:x}", trace.finalize()),
        ];
        ensure(
            actual[0] == expected[0]
                && actual[1] == expected[1]
                && (expected[2] == "-" || actual[2] == expected[2]),
            &format!("{name} full physical/O1/O2 column identity"),
        )?;
        let velocity: Vec<_> = rows.iter().map(|r| r[21]).collect();
        let oracle = oracle::taps(f64::from(c.rate), &velocity);
        let mut errors = [[0.0_f64; 4]; 3];
        for (i, (r, [h, a, l])) in rows.iter().zip(oracle).enumerate() {
            for (j, d) in [0., 0.5, 1.].into_iter().enumerate() {
                for (col, (&actual, wanted)) in [r[22], r[23], r[24], r[25 + j]]
                    .iter()
                    .zip([h, a, l, (1. - 0.025 * d) * l + 0.025 * d * a])
                    .enumerate()
                {
                    let e = (actual - wanted).abs();
                    if !(0.0..=1e-9).contains(&e) {
                        return Err(format!(
                            "{name} depth={d} frame={i} tap={col} {actual} vs {wanted}"
                        )
                        .into());
                    }
                    errors[j][col] = errors[j][col].max(e);
                }
            }
        }
        let n = rows.len();
        if expected[2] != "-" {
            fs::write(traces.join(format!("{name}.f64le")), raw)?;
            let hashes: BTreeMap<_, _> = [3, 11, 21, 22, 24, 25, 26, 27]
                .into_iter()
                .zip(taps)
                .map(|(i, h)| (i.to_string(), format!("{:x}", h.finalize())))
                .collect();
            evidence::atomic(
                &traces.join(format!("{name}.json")),
                &json!({"fixture":name,"rate":c.rate,"pitch":c.hz,"epoch":1,"start":0,"end":n,"count":n,"window":[n-c.rate as usize,n],"source":actual[2],"executable":producer,"invocation":["--o2-guide",c.rate.to_string()],"controls":c.controls_sha256,"taps":hashes,"commits":[n,1,0,1,n,n]}),
            )?;
            for (label, j) in [("M5", 11), ("off", 25), ("factory", 26)] {
                let x: Vec<_> = rows[..(c.seconds * c.rate) as usize]
                    .iter()
                    .map(|r| r[j] as f32)
                    .collect();
                fs::write(
                    audio.join(format!("{name}-{label}.wav")),
                    float_wav(c.rate, &x),
                )?;
            }
        }
        result.insert(name,json!({"ticks":n,"commits":n,"calls":n,"original_sha256":actual[0],"observed_sha256":actual[1],"full_sha256":actual[2],"errors":errors}));
    }
    for line in include_str!("../../../validation/fixtures/stage07_audio_sha256.tsv").lines() {
        let (digest, name) = line.split_once("  ").ok_or("audio manifest format")?;
        if name.starts_with("audio/") {
            ensure(
                evidence::digest(&out.join(name))? == digest,
                &format!("engineering audio {name}"),
            )?;
        }
    }
    Ok(json!({"cases":result,"producer_sha256":producer}))
}
