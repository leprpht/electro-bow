//! Grade an O2 claim against all the measurements made by one fresh group of child processes.
use crate::{Result, ensure, evidence, publication};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
const KEY: &str = "48000/196/0.1/1.8/0.5";
fn array(v: &Value) -> Result<&Vec<Value>> {
    v.as_array()
        .ok_or_else(|| "expected measurement array".into())
}
fn map(v: &Value) -> Result<&serde_json::Map<String, Value>> {
    v.as_object()
        .ok_or_else(|| "expected measurement object".into())
}
fn grade(claim: &Value, fresh: &Value) -> Result<()> {
    for key in map(fresh)?.keys() {
        ensure(
            claim.get(key).is_some(),
            &format!("missing evidence ['{key}']"),
        )?;
    }
    let rows = map(&claim["v17"])?;
    ensure(rows.keys().eq(map(&fresh["v17"])?.keys()), "V17 inventory")?;
    let corners = array(&claim["corners"])?;
    ensure(
        corners.len() == 384 && corners == array(&fresh["corners"])?,
        "corners inventory",
    )?;
    for (name, row) in rows {
        let original = &fresh["v17"][name];
        ensure(
            map(&row["passes"])?
                .keys()
                .eq(map(&original["passes"])?.keys()),
            "V17 predicate inventory",
        )?;
        if original["passes"]["factory_eta"].is_boolean() {
            ensure(
                row["passes"]["bridge_anchor"] == true && row["spectral"].is_object(),
                "ineligible M5 bridge anchor or wet spectrum",
            )?;
            ensure(
                row["factory_difference_over_off"]
                    .as_f64()
                    .is_some_and(f64::is_finite),
                "factory residual missing/nonfinite value",
            )?;
        }
    }
    let sweeps = array(&claim["sweep"])?;
    for (r, expected) in sweeps.iter().zip(array(&fresh["sweep"])?) {
        ensure(
            array(&r["peaks"])?
                .iter()
                .all(|p| p["dB"].as_f64().is_some_and(|v| v <= 6.)),
            "sweep above +6 dB",
        )?;
        ensure(r == expected, "tap-derived sweep")?;
    }
    for (r, expected) in array(&claim["electrical"])?
        .iter()
        .zip(array(&fresh["electrical"])?)
    {
        for (depth, original) in array(&r["response"])?
            .iter()
            .zip(array(&expected["response"])?)
        {
            for (point, p) in array(&depth["points"])?
                .iter()
                .zip(array(&original["points"])?)
            {
                ensure(
                    point["real"] == p["real"] && point["imag"] == p["imag"],
                    "complex probe",
                )?;
                ensure(
                    point["phase_deg"] == p["phase_deg"] && point["delay_s"] == p["delay_s"],
                    "phase",
                )?;
                ensure(
                    point["load_magnitude"] == p["load_magnitude"],
                    "load magnitude",
                )?;
            }
        }
    }
    for (r, expected) in array(&claim["local_timing"]["rates"])?
        .iter()
        .zip(array(&fresh["local_timing"]["rates"])?)
    {
        for v in array(&r["records"])? {
            if v["workload"] == "near-denormal" {
                ensure(
                    v["terminal"].as_f64().is_some_and(f64::is_subnormal),
                    "active subnormal workload",
                )?;
            }
        }
        ensure(r == expected, "measured timing record")?;
    }
    ensure(
        claim == fresh,
        "complete evidence differs from fresh producer measurements",
    )
}
pub fn run(root: &Path) -> Result<Value> {
    let get = |suite: &str| -> Result<Value> {
        let report = publication::current_artifact(root, suite, "evidence.json")?;
        Ok(evidence::strict(&fs::read(report)?)?["results"].take())
    };
    let audio = get("o2-acoustic")?;
    let mut controls: BTreeMap<String, Value> =
        serde_json::from_value(get("o2-records")?["controls"].take())?;
    let fresh = json!({"high_band":audio["high_band"],"onset_release":audio["onset_release"],"inputs":audio["sources"],"replay":get("o2-carrier")?,"corners":audio["corners"],"v17":audio["v17"],"listening":audio["listening"],"nominal":audio["nominal_sha256"],"witness":audio["witness"],"sweep":get("o2-sweep")?["rows"],"electrical":get("o2-bench")?["rates"],"local_timing":get("o2-timing")?});
    grade(&fresh, &fresh)?;
    let expected: BTreeMap<String, String> = serde_json::from_str(include_str!(
        "../../../validation/fixtures/o2_negative_predicates.json"
    ))?;
    let names = expected
        .keys()
        .filter(|k| !controls.contains_key(*k))
        .cloned()
        .collect::<Vec<_>>();
    for name in names {
        let mut copy = fresh.clone();
        let want = match name.as_str() {
            key if key.starts_with("removed ") => {
                let section = &key[8..];
                copy.as_object_mut().unwrap().remove(section);
                expected[&name].as_str()
            }
            "omitted required row" => {
                copy["v17"].as_object_mut().unwrap().remove(KEY);
                "V17 inventory"
            }
            "duplicate corner" => {
                let x = copy["corners"][0].clone();
                copy["corners"].as_array_mut().unwrap().push(x);
                "corners inventory"
            }
            "null measurement" => {
                copy["v17"][KEY]["factory_difference_over_off"] = Value::Null;
                "factory residual missing/nonfinite value"
            }
            "nonfinite measurement" => {
                copy["v17"][KEY]["factory_difference_over_off"] = "NONFINITE_CONTROL".into();
                let json = serde_json::to_string(&copy)?.replace("\"NONFINITE_CONTROL\"", "NaN");
                let err = evidence::strict(json.as_bytes())
                    .err()
                    .ok_or("accepted nonfinite measurement")?;
                controls.insert(
                    name,
                    json!({"status":1,"first":err.to_string(),"boundary":"strict JSON ingestion"}),
                );
                continue;
            }
            "missing predicate" => {
                copy["v17"][KEY]["passes"]
                    .as_object_mut()
                    .unwrap()
                    .remove("factory_eta");
                "V17 predicate inventory"
            }
            "voiced off-target anchor" => {
                let t = (0..48000)
                    .map(|i| {
                        let m = (std::f64::consts::TAU * 196. * i as f64 / 48000.).sin();
                        (m, 0.5 * m + if i % 2 == 0 { 0.6 } else { -0.6 })
                    })
                    .collect::<Vec<_>>();
                let raw = t
                    .iter()
                    .map(|(m, a)| {
                        let mut row = [*m; 30];
                        row[11] = *a;
                        row
                    })
                    .collect::<Vec<_>>();
                let wet = t.iter().map(|(m, _)| [*m; 4]).collect::<Vec<_>>();
                copy["v17"][KEY] = crate::identity::stationary::grade(&raw, &wet, 48000, 196., 1)?;
                "ineligible M5 bridge anchor or wet spectrum"
            }
            "reported sweep peak" => {
                copy["sweep"][0]["peaks"] = json!([{"hz":3000.,"dB":8.,"width_hz":17.}]);
                "sweep above +6 dB"
            }
            "fabricated factory sweep" => {
                copy["sweep"][1]["complex_error"] = 0.into();
                copy["sweep"][1]["peaks"] = json!([{"hz":65.,"dB":-100.,"width_hz":null}]);
                "tap-derived sweep"
            }
            "complex cutoff" => {
                let p = &mut copy["electrical"][4]["response"][1]["points"][6];
                p["real"] = 123.into();
                p["imag"] = (-456).into();
                "complex probe"
            }
            "wrong phase" => {
                copy["electrical"][4]["response"][1]["points"][6]["phase_deg"] = 999.into();
                "phase"
            }
            "wrong load magnitude" => {
                copy["electrical"][4]["response"][1]["points"][6]["load_magnitude"] = (-100).into();
                "load magnitude"
            }
            "fabricated Nyquist phase/delay" => {
                let p = &mut copy["electrical"][4]["response"][1]["points"][9];
                p["phase_deg"] = 900.into();
                p["delay_s"] = 100.into();
                "phase"
            }
            "cross-rate timing" => {
                copy["local_timing"]["rates"][2] = copy["local_timing"]["rates"][0].clone();
                "measured timing record"
            }
            "flushed subnormal" => {
                for row in copy["local_timing"]["rates"][1]["records"]
                    .as_array_mut()
                    .unwrap()
                {
                    if row["workload"] == "near-denormal" {
                        row["terminal"] = 0.into();
                    }
                }
                "active subnormal workload"
            }
            _ => return Err(format!("unimplemented O2 control {name}").into()),
        };
        let e = grade(&copy, &fresh)
            .err()
            .ok_or_else(|| format!("surviving {name}"))?
            .to_string();
        ensure(e == want, &format!("{name} wrong first failure: {e}"))?;
        controls.insert(name, json!({"status":1,"first":e}));
    }
    ensure(
        controls.len() == 54 && controls.keys().eq(expected.keys()),
        "complete O2 negative inventory",
    )?;
    Ok(json!({"measurements":fresh,"negative_controls":controls}))
}
