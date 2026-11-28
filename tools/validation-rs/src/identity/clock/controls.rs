//! Independent predicates for Stage 08 capture window and publication claims.
use super::*;
fn publishes(r: &Value) -> bool {
    r["actions"]
        .as_array()
        .is_some_and(|a| a.iter().any(|v| v == "publish"))
}
fn tick(i: usize) -> usize {
    240 * (i + 1) - 1
}
fn grade(c: &Case, rows: &[Value]) -> Result<()> {
    let mut by = BTreeMap::new();
    for r in rows {
        let t = r["tick"].as_u64().ok_or("clock tick")? as usize;
        ensure(by.insert(t, r).is_none(), "duplicate clock tick")?;
    }
    let at = |n| by.get(&tick(n)).copied().ok_or("missing due event");
    ensure(
        at(38)?["event"] == "pick" && !publishes(at(38)?),
        "repick priority",
    )?;
    ensure(
        at(95)?["event"] == "reset" && at(95)?["live"].is_null(),
        "reset priority",
    )?;
    for (t, r) in &by {
        if let Some(w) = r["window"].as_array() {
            let w: Vec<i64> = w
                .iter()
                .map(|v| v.as_i64().ok_or("window integer"))
                .collect::<std::result::Result<_, _>>()?;
            ensure(
                w.len() == 5
                    && w[0] >= 0
                    && w[1] == *t as i64
                    && w[2] <= 2 * w[1]
                    && w[1] <= w[3]
                    && w[3] < w[4],
                "causal capture",
            )?;
            let end = *t as i64;
            let count = (64000. / c.frames.get(t).ok_or("capture frame")?.hz + 0.5).floor() as i64;
            let first = end - 3 * (count - 1);
            ensure(
                w == [
                    first - 96,
                    end,
                    first + end - 96,
                    end + (count + 4) / 5 - 1,
                    end + 240,
                ],
                "FIR window/support clock",
            )?;
        }
        if publishes(r) {
            let last = t
                .checked_sub(240)
                .and_then(|d| by.get(&d))
                .ok_or("missing previous clock")?;
            ensure(
                last["window"].get(4) == Some(&r["tick"]) && last["owner"] == r["owner"],
                "stale publication",
            )?;
        }
    }
    for i in [18, 19, 23, 28, 31, 34, 59, 91] {
        ensure(!publishes(at(i)?), "invalid refresh")?;
        if i < 59 {
            ensure(
                at(i)?["held"] == at(i - 1)?["held"],
                "unsupported source changed held identity",
            )?;
        }
    }
    let changed = |v: &Value| {
        v.as_array()
            .is_some_and(|a| a.iter().any(|x| x.as_f64().is_some_and(|n| n != 0.)))
    };
    let first = rows
        .iter()
        .find(|r| r["window"].is_array())
        .ok_or("first window")?;
    ensure(
        first["tick"] == 2399
            && first["window"].as_array().unwrap()[..3] == [json!(1325), json!(2399), json!(3724)],
        "first eligible capture",
    )?;
    ensure(
        changed(&at(38)?["held"]) && !changed(&at(38)?["published"]),
        "repick held and published",
    )?;
    ensure(
        !changed(&at(95)?["held"]) && at(59)?["phase"] == "release" && at(60)?["live"].is_null(),
        "reset and disposal",
    )?;
    ensure(
        publishes(at(10)?) && at(10)?["consumed"] == at(10)?["live"],
        "coincident launch",
    )
}
pub(super) fn run(cases: &[Case]) -> Result<Value> {
    let [main, before, suffix, stale, late, wrong] = cases else {
        return Err("clock fixture inventory".into());
    };
    let rows = replay(main, &[127])?;
    grade(main, &rows)?;
    let cutoff = tick(15);
    ensure(
        replay(suffix, &[127])?
            .iter()
            .take_while(|r| r["tick"].as_u64().unwrap() <= cutoff as u64)
            .eq(rows
                .iter()
                .take_while(|r| r["tick"].as_u64().unwrap() <= cutoff as u64)),
        "future input changes past output",
    )?;
    let prior = replay(before, &[127])?;
    ensure(
        prior.iter().any(|r| {
            r["tick"] == tick(13) + 1
                && r["live"] == json!([0, 1])
                && r["held"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|v| v.as_f64() != Some(0.)))
        }),
        "publication before launch",
    )?;
    let mut rejected = BTreeMap::new();
    ensure(
        replay(stale, &[127])?.iter().all(|r| r["live"].is_null()),
        "wrong launch token",
    )?;
    rejected.insert(
        "wrong launch token",
        "stale confirmation cannot promote a gesture".to_string(),
    );
    for (name, index, expected) in [
        ("wrong event order", 38, "repick priority"),
        ("wrong capture window", 14, "FIR window/support clock"),
        ("future input", 10, "causal capture"),
        ("invalid refresh", 18, "invalid refresh"),
        ("stale owner", 14, "stale publication"),
    ] {
        let mut altered = rows.clone();
        let r = altered
            .iter_mut()
            .find(|r| r["tick"] == tick(index))
            .ok_or("missing altered tick")?;
        match name {
            "wrong event order" => r["event"] = "mute".into(),
            "wrong capture window" => {
                r["window"][0] = (r["window"][0].as_i64().ok_or("window start")? + 3).into()
            }
            "future input" => r["window"][1] = (tick(index) + 1).into(),
            "invalid refresh" => r["actions"]
                .as_array_mut()
                .ok_or("actions")?
                .push("publish".into()),
            _ => r["owner"] = json!([0, -2]),
        }
        let err = grade(main, &altered)
            .err()
            .ok_or("missed defective clock record")?
            .to_string();
        ensure(err.contains(expected), &format!("{name} failed at {err}"))?;
        rejected.insert(name, err);
    }
    for (name, c) in [("late", late), ("wrong epoch", wrong)] {
        ensure(
            c.defect.as_deref() == Some(name) && replay(c, &[127])?.iter().all(|r| !publishes(r)),
            "obsolete publication",
        )?;
        rejected.insert(name, "no obsolete publication".to_string());
    }
    Ok(json!(rejected))
}
