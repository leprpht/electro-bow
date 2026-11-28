use super::*;
use crate::identity::{stationary as revised, sweeps};

fn complex() -> Result<Value> {
    let mut result = BTreeMap::new();
    let g = std::array::from_fn(|j| 10f64.powf(if j % 2 == 0 { -0.25 } else { 0.25 }) - 1.);
    for rate in [32000, 48000, 192000] {
        for f in [82.41, 196., 1318.51] {
            let mut bank = Bank::new(f64::from(rate))?;
            let mut impulse = Vec::new();
            let mut row = [0.; 17];
            row[0] = f;
            row[5..].copy_from_slice(&g);
            for i in 0..rate {
                row[1] = f64::from(i == 0);
                impulse.push(row[1] + bank.apply(&row)?[0]);
            }
            let err = numerics::spectrum(&impulse)
                .iter()
                .enumerate()
                .map(|(i, v)| (*v - sweeps::transfer(rate.into(), f, i as f64, &g)).norm())
                .fold(0., f64::max);
            ensure(err <= 1e-9, "historical overlapping impulse response")?;
            result.insert(format!("{rate}/{f}"), err);
        }
    }
    Ok(json!(result))
}
pub(super) fn run(exe: &Path) -> Result<Value> {
    let contract: revised::Contract =
        serde_json::from_str(include_str!("../../../contracts/stationary.json"))?;
    let carriers = carrier::cases()?;
    let mut rows = BTreeMap::new();
    for c in contract.cases {
        let raw = carrier::render(exe, &carriers[&c.physical])?;
        let (constant, updates) = &contract.schedules[&c.schedule];
        let y = render(
            &raw,
            &vec![c.hz; raw.len()],
            constant.is_none().then_some(updates),
            false,
            c.rate,
            constant.unwrap_or([0.; 12]),
        )?;
        let neutral = raw
            .iter()
            .map(|r| [r[25], r[26], r[27], r[26]])
            .collect::<Vec<_>>();
        let mut neutral_rows = Vec::new();
        let mut depths = Vec::new();
        for j in 0..3 {
            neutral_rows.push(revised::grade(&raw, &neutral, c.rate, c.hz, j)?);
            let row = revised::grade(&raw, &y.wet, c.rate, c.hz, j)?;
            ensure(
                row["passes"]
                    .as_object()
                    .unwrap()
                    .values()
                    .all(|v| v != false),
                "legacy stationary V17",
            )?;
            depths.push(row);
        }
        let effective = y.final_coeff.map(|g| g * y.scale.last().unwrap());
        let peaks = sweeps::analytical(c.rate.into(), c.hz, &effective)?;
        rows.insert(c.case,json!({"neutral":neutral_rows,"depths":depths,"policy":policy(&y),"sweep":peaks,"requested_dB":y.final_coeff.map(|g|20.*(1.+g).log10()),"accepted_dB":effective.map(|g|20.*(1.+g).log10())}));
    }
    let sweep = sweeps::measured(exe, &rows)?;
    let expression = sweeps::expression(&rows)?;
    for r in rows.values_mut() {
        r["neutral_depths"] = r.as_object_mut().unwrap().remove("neutral").unwrap();
    }
    let mut corners = BTreeMap::new();
    let direct = acoustic::pluck(440.)
        .into_iter()
        .map(f64::abs)
        .fold(0., f64::max);
    for v in [0.06, 0.16] {
        for r in [1.2, 2.4] {
            let raw = carrier::render(exe, &carriers[&format!("steady_48000_440_R{r}_V{v}_3s")])?;
            for name in ["neutral", "upper", "lower", "inversion"] {
                let wet = if name == "neutral" {
                    raw.iter().map(|a| [a[25], a[26], a[27], a[26]]).collect()
                } else {
                    render(
                        &raw,
                        &vec![440.; raw.len()],
                        None,
                        false,
                        48000,
                        contract.schedules[name].0.ok_or("constant corner")?,
                    )?
                    .wet
                };
                for (j, d) in ["0", "0.5", "1"].iter().enumerate() {
                    let w = wet.iter().map(|a| a[j].abs()).fold(0., f64::max);
                    for g in [0.08, 1.4] {
                        for ch in [0, 1] {
                            for trim in [0, 2] {
                                let peak = trim as f64 * if ch == 0 { direct } else { g * w };
                                corners.insert(format!("{v}/{r}/{name}/{d}/{g}/{ch}/{trim}"),json!({"peak":peak,"unity_wet_peak":w,"demand_dB":(20.*(peak/0.98).max(1e-12).log10()).max(0.)}));
                            }
                        }
                    }
                }
            }
        }
    }
    ensure(corners.len() == 384 && rows.len() == 36, "legacy gain grid")?;
    Ok(
        json!({"stationary":rows,"corners":corners,"complex":complex()?,"expression":expression,"measured_sweep":sweep}),
    )
}
