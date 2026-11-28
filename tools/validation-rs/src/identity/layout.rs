//! Reproduce the submitted storage arithmetic and expose Review 8.2's unassigned fields.
use crate::{Result, ensure};
use serde_json::{Value, json};
use std::collections::BTreeMap;
type Groups = BTreeMap<String, Vec<(String, usize)>>;
fn group(fields: &[(String, usize)], overrides: &[usize]) -> (BTreeMap<String, [usize; 2]>, usize) {
    let mut offset = 0;
    let mut rows = BTreeMap::new();
    for (i, (name, declared)) in fields.iter().enumerate() {
        let size = *overrides.get(i).unwrap_or(declared);
        rows.insert(name.clone(), [offset, size]);
        offset += size;
    }
    (rows, offset)
}
pub fn run() -> Result<Value> {
    let groups: Groups = serde_json::from_str(include_str!("../../contracts/layout.json"))?;
    let mut rates = vec![32000f64, 192000.];
    for at in (40000..=192000).step_by(8000) {
        rates.extend([f64::from(at).next_down(), f64::from(at)]);
    }
    let mut peak = 0;
    let mut worst = Value::Null;
    let mut rows = vec![];
    for fs in rates {
        let da = (fs / 8000.).floor() as usize;
        let di = (fs / 16000.).floor() as usize;
        let w = (fs / da as f64 / 65.).ceil() as usize;
        let m = (4. * fs / di as f64 / 65.).ceil() as usize;
        let h = (0.005 * fs / di as f64 + 0.5).floor() as usize;
        let (shared, s) = group(&groups["shared"], &[8 * (16 * da + 1), 8 * (16 * di + 1)]);
        let (lane, l) = group(
            &groups["lane"],
            &[
                8 * (32 * da + 1),
                8 * (2 * w + 2),
                16 * (w + 2),
                8 * (m + 1 + h + 2),
            ],
        );
        let controls = s + 2 * l;
        let (full, total) = group(&groups["full_layout"], &[controls]);
        ensure(
            controls <= 65536 && total <= 838860,
            "submitted prepared payload",
        )?;
        if total > peak {
            peak = total;
            worst = json!({"fs":fs,"shared":shared,"lane":lane,"analysis_bytes":controls,"shared_bytes":s,"lane_bytes":l,"full_layout":full,"total_bytes":total});
        }
        rows.push(json!({"rate":fs,"analysis":controls,"total":total}));
    }
    ensure(
        worst["analysis_bytes"] == 61376 && peak == 726976 && worst["fs"] == 192000f64.next_down(),
        "R82 submitted worst capacity",
    )?;
    let extra = 2 * (24 * (9 - 5) * 8 + 14 * 8);
    Ok(
        json!({"rate_grid":rows,"maximum":worst,"literal_support_and_receipt_projection":{"analysis_bytes":61376+extra,"total_bytes":peak+extra},"complete_field_ownership_verified":false,"measured_allocations":null}),
    )
}

mod prepared;
pub use prepared::prepared;
