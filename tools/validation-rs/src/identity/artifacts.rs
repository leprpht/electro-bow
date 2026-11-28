//! Historical NumPy little-endian binary64 interchange and independently bound Rust replay records.
use crate::{Result, ensure, evidence};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{BufWriter, Write},
    path::Path,
};
pub(super) fn array(
    name: &str,
    rows: usize,
    cols: Option<usize>,
    words: impl IntoIterator<Item = f64>,
) -> Result<(String, Vec<u8>)> {
    let shape = match cols {
        Some(c) => format!("({rows}, {c})"),
        None => format!("({rows},)"),
    };
    let header = format!("{{'descr': '<f8', 'fortran_order': False, 'shape': {shape}, }}");
    let length = (10 + header.len() + 1).next_multiple_of(64);
    let count = rows.checked_mul(cols.unwrap_or(1)).ok_or("NPY shape")?;
    let mut bytes = Vec::with_capacity(length + 8 * count);
    bytes.extend(b"\x93NUMPY\x01\x00");
    bytes.extend(u16::try_from(length - 10)?.to_le_bytes());
    bytes.extend(header.bytes());
    bytes.resize(length - 1, b' ');
    bytes.push(b'\n');
    for v in words {
        bytes.extend(v.to_le_bytes());
    }
    ensure(
        bytes.len() == length + count * 8,
        "NPY declared sample count",
    )?;
    Ok((format!("{name}.npy"), bytes))
}
pub(super) fn matrix<const N: usize>(name: &str, rows: &[[f64; N]]) -> Result<(String, Vec<u8>)> {
    array(name, rows.len(), Some(N), rows.iter().flatten().copied())
}
pub(super) fn write_rows<const N: usize>(path: &Path, rows: &[[f64; N]]) -> Result<()> {
    let mut file = BufWriter::new(fs::File::create(path)?);
    for r in rows {
        file.write_all(r.map(f64::to_le_bytes).as_flattened())?;
    }
    file.flush()?;
    Ok(())
}
pub(super) fn physical(
    dir: &Path,
    name: &str,
    exe: &Path,
    rate: u32,
    controls: &[u8],
    rows: &[[f64; 30]],
) -> Result<()> {
    fs::create_dir_all(dir)?;
    let path = dir.join(format!("{name}.f64le"));
    write_rows(&path, rows)?;
    let mut taps = BTreeMap::new();
    for j in [3, 11, 21, 22, 24, 25, 26, 27] {
        let mut h = Sha256::new();
        for r in rows {
            h.update(r[j].to_le_bytes());
        }
        taps.insert(j, format!("{:x}", h.finalize()));
    }
    let n = rows.len();
    let record = json!({"fixture":name,"rate":rate,"pitch":196.0,"epoch":1,"start":0,"end":n,"count":n,"window":n.checked_sub(rate as usize).map(|start|[start,n]),"source":evidence::digest(&path)?,"executable":evidence::digest(exe)?,"invocation":["--o2-guide",rate.to_string()],"controls":format!("{:x}",Sha256::digest(controls)),"taps":taps,"commits":[n,1,0,1,n,n]});
    evidence::atomic(&path.with_extension("json"), &record)
}

pub fn inventory(root: &Path) -> Result<serde_json::Value> {
    use crate::publication;
    use std::io::Read;
    let contract = evidence::strict(include_bytes!("../../contracts/i1-inventory.json"))?;
    let mut sources = BTreeMap::new();
    for suite in [
        "i1-motion",
        "i1-stereo",
        "i1-uneven",
        "i1-sources",
        "i1-study",
        "i1-listening",
    ] {
        let receipt = publication::inspect(&root.join(suite), suite)?;
        ensure(
            receipt["status"] == "passed",
            "successful artifact producer",
        )?;
        let dir = root
            .join(suite)
            .join(receipt["run_id"].as_str().ok_or("producer run")?);
        sources.insert(suite, (dir, receipt));
    }
    let mut rows = BTreeMap::new();
    for (name, expected) in contract["artifacts"]
        .as_object()
        .ok_or("artifact contract")?
    {
        let owner = if name.starts_with("motion/") {
            "i1-motion"
        } else if name.starts_with("joined/uneven") {
            "i1-uneven"
        } else if name.starts_with("joined/") {
            "i1-stereo"
        } else {
            match name.as_str() {
                "stage-08-2-listening.zip" => "i1-listening",
                "revised-held.npz" => "i1-sources",
                _ => "i1-study",
            }
        };
        let (dir, receipt) = &sources[owner];
        let path = dir.join(name);
        let actual = receipt["artifacts"][name]
            .as_str()
            .ok_or_else(|| format!("producer missing {name}"))?;
        let mut row = json!({"producer":owner,"run":receipt["run_id"],"entry":expected["sha256"],"actual":actual});
        if name.ends_with(".json") {
            let mut old = expected["record"].clone();
            let new = evidence::strict(&fs::read(&path)?)?;
            ensure(
                new["executable"]
                    == receipt["artifacts"][format!("renderer{}", std::env::consts::EXE_SUFFIX)],
                "physical receipt replay executable",
            )?;
            old["executable"] = new["executable"].clone();
            ensure(
                old == new,
                &format!("physical clock, source and tap identities {name}"),
            )?;
            row["equivalence"] = "same measured fields, invocation-local renderer identity".into();
        } else if name == "listening.zip" {
            let mut z = zip::ZipArchive::new(fs::File::open(&path)?)?;
            let members = expected["members"]
                .as_object()
                .ok_or("historical archive inventory")?;
            ensure(z.len() == members.len(), "archive member count")?;
            let mut distances = BTreeMap::new();
            for key in members.keys() {
                let mut data = Vec::new();
                z.by_name(key)?.read_to_end(&mut data)?;
                if key == "listening-gains.json" {
                    let new = evidence::strict(&data)?;
                    let old = expected["gains"].as_object().ok_or("original gains")?;
                    ensure(
                        new.as_object().is_some_and(|r| r.keys().eq(old.keys())),
                        "complete gain manifest",
                    )?;
                    for (k, a) in old {
                        let a = a.as_f64().ok_or("original gain")?;
                        let b = new[k].as_f64().ok_or("fresh gain")?;
                        ensure((a - b).abs() <= 2e-11, "recomputed audition gain")?;
                        distances.insert(k, a.to_bits().abs_diff(b.to_bits()));
                    }
                } else {
                    ensure(
                        members[key] == format!("{:x}", Sha256::digest(&data)),
                        "audition member bytes",
                    )?;
                }
            }
            row["equivalence"] = if expected["sha256"] == actual {
                "bitwise"
            } else {
                "audio bitwise, gain serialization differs"
            }
            .into();
            row["gain_ulp_distances"] = json!(distances);
        } else {
            ensure(
                expected["sha256"] == actual,
                &format!("canonical file {name}"),
            )?;
            row["equivalence"] = "bitwise".into();
        }
        rows.insert(name, row);
    }
    ensure(rows.len() == 105, "required historical artifacts")?;
    Ok(json!({"artifacts":rows,"required_reports":contract["reports"]}))
}
