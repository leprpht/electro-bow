//! Preserve fixed engineering gain, comparison windows, blinded names and the historical archive.
use crate::{
    Result, electrical::acoustic::listening::canonical_archive, ensure, evidence,
    numerics::sum_numpy, observation::float_wav, publication,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{fs, path::Path};
const NOTE: &str = "Use only the matched folder for blind source pairing. Compare neutral and revised vibrato at fixed playback gain, follow the bend and listen through the veto and recovery. Record timestamp, route, A/B cue, transition and indistinguishable outcomes before revealing the key. No owner verdict recorded.\n";
#[derive(Serialize)]
struct Clip {
    matched: f64,
    engineering: f64,
    anchor: [usize; 2],
    fs: u32,
    source: String,
    samples: String,
}
pub fn run(out: &Path, root: &Path) -> Result<Value> {
    let archived: Value = serde_json::from_str(include_str!("../../contracts/sources.json"))?;
    let mut entries = Vec::new();
    let mut manifest = Vec::new();
    let mut source_ids = Vec::new();
    for name in [
        "previous-stationary",
        "revised-vibrato",
        "neutral-vibrato",
        "revised-bend",
        "depth-veto-recovery",
        "techs-P1-60-held",
        "techs-P2-60-held",
        "techs-P1-60-DI",
        "techs-P2-60-DI",
    ] {
        let (suite, source, anchor) = match name {
            "previous-stationary" => (
                "i1-motion",
                "steady-48000, previous off".into(),
                [24000, 48000],
            ),
            "revised-vibrato" => (
                "i1-motion",
                "vibrato-48000, automated final wet".into(),
                [24000, 48000],
            ),
            "neutral-vibrato" => (
                "i1-motion",
                "vibrato-48000, automated returned q".into(),
                [24000, 48000],
            ),
            "revised-bend" => (
                "i1-motion",
                "bend-48000, automated final wet".into(),
                [24000, 48000],
            ),
            "depth-veto-recovery" => (
                "i1-motion",
                "veto-48000, automated final wet".into(),
                [24000, 48000],
            ),
            s if s.ends_with("-held") => (
                "i1-sources",
                format!("steady-48000-196, held DI {name}"),
                [0, 48000],
            ),
            _ => (
                "i1-sources",
                "attributed original native-gain DI excerpt, attack aligned".into(),
                [2880, 10560],
            ),
        };
        let file = publication::current_artifact(root, suite, &format!("{name}.f64le"))?;
        source_ids.push(json!({"filename":name,"suite":suite,"run":file.parent().and_then(Path::file_name).and_then(|s|s.to_str())}));
        let bytes = fs::read(&file)?;
        ensure(
            bytes.len() % 8 == 0 && bytes.len() / 8 >= anchor[1],
            "listening source coverage",
        )?;
        let x = bytes
            .as_chunks::<8>()
            .0
            .iter()
            .map(|w| f64::from_le_bytes(*w))
            .collect::<Vec<_>>();
        let power = x[anchor[0]..anchor[1]]
            .iter()
            .map(|v| v * v)
            .collect::<Vec<_>>();
        let gain =
            0.1 / (power.chunks(8192).map(sum_numpy).sum::<f64>() / power.len() as f64).sqrt();
        ensure(gain.is_finite() && gain > 0., "listening normalization")?;
        let data = Clip {
            matched: gain,
            engineering: 1.,
            anchor,
            fs: 48000,
            source,
            samples: evidence::digest(&file)?,
        };
        let fields = serde_json::to_string_pretty(&data)?.replace('\n', "\n  ");
        manifest.push(format!("  \"{name}\": {fields}"));
        for (folder, k) in [("engineering", 1.), ("matched", gain)] {
            let alias = if folder == "matched" && name.ends_with("-held") {
                if name.contains("P1") {
                    "held-B"
                } else {
                    "held-A"
                }
            } else {
                name
            };
            let filename = format!("{folder}/{alias}.wav");
            let wav = float_wav(48000, &x.iter().map(|v| (v * k) as f32).collect::<Vec<_>>());
            fs::create_dir_all(out.join(folder))?;
            fs::write(out.join(&filename), &wav)?;
            entries.push((filename, wav));
        }
    }
    let contents = format!("{{\n{}\n}}", manifest.join(",\n"));
    fs::write(out.join("listening-manifest.json"), format!("{contents}\n"))?;
    fs::write(
        out.join("listening-key.json"),
        "{\n  \"held-A\": \"techs-P2-60\",\n  \"held-B\": \"techs-P1-60\"\n}\n",
    )?;
    entries.push(("manifest.json".into(), contents.into_bytes()));
    entries.push((
        "ATTRIBUTION.md".into(),
        fs::read("tools/validation/fixtures/acoustic/ATTRIBUTION.md")?,
    ));
    entries.push(("LISTEN.txt".into(), NOTE.as_bytes().to_vec()));
    canonical_archive(&entries, &out.join("stage-08-2-listening.zip"))?;
    for f in [
        "listening-manifest.json",
        "listening-key.json",
        "stage-08-2-listening.zip",
    ] {
        let actual = evidence::digest(&out.join(f))?;
        ensure(
            archived[f] == actual,
            &format!("I1 listening {f}: {actual}"),
        )?;
    }
    Ok(
        json!({"sources":source_ids,"clips":9,"zip":archived["stage-08-2-listening.zip"],"owner_verdict":null}),
    )
}
