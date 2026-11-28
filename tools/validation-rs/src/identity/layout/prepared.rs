//! Proposed contiguous stereo arena. Every row describes a disjoint range;
//! the same builder computes offsets and capacity at each decimator boundary.
use super::super::candidate::{Candidate, field_offsets};
use crate::{Result, ensure};
use serde_json::{Value, json};
use std::mem::{align_of, size_of};
#[derive(Default)]
struct Arena {
    end: usize,
    rows: Vec<Value>,
}
impl Arena {
    fn field<T>(&mut self, owner: &str, names: &str, count: usize, life: &str) {
        for name in names.split_ascii_whitespace() {
            let alignment = align_of::<T>();
            let start = self.end.next_multiple_of(alignment);
            self.rows.push(json!({"owner":owner,"field":name,"type":std::any::type_name::<T>(),"count":count,"offset":start,"alignment":alignment,"bytes":count*size_of::<T>(),"lifetime":life}));
            self.end = start + count * size_of::<T>();
        }
    }
}
fn arena(fs: f64) -> (Arena, usize) {
    let da = (fs / 8000.).floor() as usize;
    let di = (fs / 16000.).floor() as usize;
    let w = (fs / da as f64 / 65.).ceil() as usize;
    let m = (4. * fs / di as f64 / 65.).ceil() as usize;
    let h = (0.005 * fs / di as f64).round() as usize;
    let mut a = Arena::default();
    a.field::<f64>("shared", "a1_fir_half", 16 * da + 1, "prepare");
    a.field::<f64>("shared", "i1_fir_half", 16 * di + 1, "prepare");
    a.field::<f64>("shared", "o2_coefficients", 6, "prepare");
    a.field::<f64>("shared","fs hp fast_alpha slow_alpha brightness_alpha feature_alpha coefficient_alpha control_alpha",1,"prepare");
    a.field::<u64>("shared","a2_decimation i1_decimation a2_window maximum_lag a2_hop i1_hop projection_budget ramp_ticks",1,"prepare");
    a.field::<u64>("transport", "generation origin tick reset", 1, "stream");
    a.field::<f64>("controls", "current requested", 6, "stream");
    for lane in ["left", "right"] {
        // The two immutable convolution kernels read the same chronological HP
        // samples at different cursors. The longer A1 support determines capacity.
        a.field::<f64>(lane, "source_hp_history", 32 * da + 1, "transport");
        a.field::<f64>(lane, "a2_history", 2 * w + 2, "transport");
        a.field::<f64>(
            lane,
            "lag_difference normalized_difference",
            w + 2,
            "A2 rolling lag sums and hop normalization",
        );
        a.field::<f64>(lane, "source_decimated_history", m + 1 + h + 2, "transport");
        a.field::<u64>(lane,"hp_write hp_valid a1_decimation_cursor i1_decimation_cursor a2_write a2_valid source_write source_valid a2_next_hop source_next_hop lag_rebase_cursor",1,"transport");
        a.field::<f64>(lane,"hp_input_previous hp_previous fast_power slow_power bright_low_power bright_high_power",1,"transport");
        a.field::<f64>(
            lane,
            "signed_peak",
            11,
            "ten completed buckets and current partial",
        );
        a.field::<u64>(
            lane,
            "peak_tick",
            11,
            "ten completed buckets and current partial",
        );
        a.field::<u64>(
            lane,
            "peak_bucket peak_bucket_end peak_bucket_start",
            1,
            "transport",
        );
        a.field::<u64>(lane, "support_head support_valid", 1, "acquisition");
        a.field::<u64>(
            lane,
            "support_tick support_generation support_serial support_flags",
            24,
            "last 24 due support frames",
        );
        a.field::<f64>(
            lane,
            "support_hz support_quality support_amplitude support_previous_floor support_movement",
            24,
            "last 24 due support frames",
        );
        a.field::<f64>(lane,"floor previous_amplitude previous_slow pick_peak pickup_orientation last_hz change_hz slow_level captured_u live_u captured_b live_b signed_gain speed speed_goal force force_goal energy energy_origin energy_target",1,"A3 lane");
        a.field::<u64>(lane,"gesture_mode last_pick last_support strong_count change_count weak_count ramp_index ramp_duration serial_counter epoch_counter",1,"A3 lane");
        a.field::<u64>(lane,"pending_tag pending_generation pending_serial pending_anchor confirmed_tag confirmed_generation confirmed_serial confirmed_through active_tag active_generation active_epoch active_source_generation active_source_serial",1,"acquisition or physical epoch");
        a.field::<u64>(
            lane,
            "job_tag job_generation job_serial job_snapshot job_due job_first job_cursor job_count",
            1,
            "running projection",
        );
        a.field::<f64>(lane,"job_hz job_floor job_alpha job_x_sum job_weight_sum hann_cos hann_sin hann_step_cos hann_step_sin",1,"running projection");
        a.field::<[f64; 2]>(
            lane,
            "projection_xw projection_w projection_phasor projection_rotation",
            12,
            "running projection",
        );
        for slot in ["completed", "published", "held"] {
            let owner = format!("{lane}.{slot}");
            a.field::<f64>(&owner, "feature_db", 12, slot);
            a.field::<u64>(&owner, "tag generation serial snapshot due", 1, slot);
        }
        a.field::<f64>(lane, "coefficient", 12, "physical epoch");
        a.field::<u64>(
            lane,
            "receipt_tag receipt_generation receipt_epoch receipt_issued receipt_through",
            1,
            "until revocation, expiry or disposal",
        );
        a.field::<f64>(lane, "receipt_rate", 1, "receipt");
        a.field::<f64>(
            lane,
            "receipt_hz receipt_speed receipt_force receipt_depth",
            2,
            "receipt inclusive ranges",
        );
        a.field::<u64>(lane,"event_generation gesture_tag confirm_tag confirm_generation confirm_serial confirm_through dispose_tag dispose_generation dispose_epoch launch_tag launch_generation launch_epoch launch_source_generation launch_serial a2_stamp_tag a2_stamp_generation a2_stamp_end i1_stamp_tag i1_stamp_generation i1_stamp_end receipt_update_tag",1,"current host frame");
        a.field::<f64>(lane,"dispose_energy launch_energy requested_hz requested_speed requested_force requested_depth",1,"current host frame");
        a.field::<[f64; 4]>(lane, "o2_x1_y1_z1_z2", 4, "physical epoch");
        a.field::<Candidate>(lane, "epoch", 1, "physical epoch");
        a.field::<f64>(
            lane,
            "last_finite_E last_finite_P",
            1,
            "last successful tick, frozen through fault fade",
        );
        a.field::<u64>(
            lane,
            "fault_site failed_generation failed_epoch fade_tag fade_index fade_length",
            1,
            "retirement through terminal fade",
        );
    }
    // Exclusive serial stereo borrow allows both lanes to use these work arrays.
    a.field::<f64>("prepared_work", "cholesky", 21, "one close");
    a.field::<f64>("prepared_work", "basis trial_vector", 6, "one tick");
    a.field::<f64>("prepared_work", "q delta realized", 4, "one tick");
    a.field::<f64>(
        "prepared_work",
        "magnitude reliability gain contrast",
        12,
        "projection finalization",
    );
    let analysis = a.end;
    a.field::<f64>(
        "guide",
        "bridge_L nut_L bridge_R nut_R",
        4096,
        "prepared physical rings",
    );
    a.field::<[f32; 8192]>("stream", "split_bands", 8, "current host block");
    a.field::<[f32; 8192]>("stream", "raw_and_preswell", 6, "current host block");
    a.field::<[f32; 8192]>("host", "wet", 2, "current host block");
    for lane in ["guide.left", "guide.right"] {
        a.field::<f64>(
            lane,
            "rate dc_coefficient previous_pickup previous_dc",
            1,
            "guide",
        );
        a.field::<f64>(lane, "friction_mu", 10, "prepare");
        a.field::<f64>(lane, "loss", 2, "guide");
        a.field::<u64>(lane, "mode live", 1, "guide");
        a.field::<u64>(
            lane,
            "ring_offset ring_capacity ring_arm ring_write ring_count seed_tag",
            2,
            "guide rings",
        );
        a.field::<f64>(lane, "seed_period seed_amplitude", 2, "guide rings");
    }
    a.field::<u64>(
        "stream",
        "capacity channels valid_frames analysis_offsets",
        1,
        "stream",
    );
    a.field::<u64>(
        "stream",
        "buffer_offset buffer_length",
        16,
        "eight bands, six raw/preswell and two host channels",
    );
    a.field::<f64>("split", "frequencies coefficients", 3, "prepare");
    a.field::<f64>("split", "filter_history", 6, "stream");
    a.field::<f64>(
        "attack",
        "samples_per_ms rise release smoothing time_constant",
        1,
        "stream",
    );
    a.field::<f64>("attack", "held fast bowed", 2, "stream");
    a.field::<u64>("attack", "initialized", 1, "stream");
    a.field::<f64>(
        "output",
        "trim trim_step linked_gain release meter_release",
        1,
        "stream",
    );
    a.field::<f64>("output", "meter_level", 2, "stream");
    a.field::<u64>("output", "trim_initialized", 1, "stream");
    (a, analysis)
}
pub fn prepared() -> Result<Value> {
    let mut rates = vec![32000., 192000.];
    for rate in (40000..=192000).step_by(8000) {
        rates.extend([f64::from(rate).next_down(), f64::from(rate)]);
    }
    let mut worst = 0;
    let mut data = Value::Null;
    let mut grid = Vec::new();
    for rate in rates {
        let (a, analysis) = arena(rate);
        ensure(
            analysis <= 65536 && a.end <= 838860,
            "exact proposed stereo capacity",
        )?;
        grid.push(json!([rate, analysis, a.end]));
        if a.end > worst {
            worst = a.end;
            data =
                json!({"rate":rate,"analysis_bytes":analysis,"total_bytes":a.end,"fields":a.rows});
        }
    }
    let mut stack = Arena::default();
    stack.field::<f64>("tick", "carrier_commit", 22, "one lane");
    stack.field::<f64>("tick", "o2_returns", 4, "one lane");
    stack.field::<f64>(
        "tick",
        "fit_sum fit_magnitude term h1 h2 residual power paired candidate weight",
        1,
        "one lane",
    );
    stack.field::<f64>(
        "tick",
        "normalized signed_gain signed_wet energy product character direct mix",
        1,
        "one lane",
    );
    stack.field::<u64>(
        "tick",
        "tick generation physical_epoch lane slot basis_index order path_index",
        1,
        "one lane",
    );
    stack.field::<f32>("tick", "converted", 2, "one frame");
    ensure(
        data["rate"] == 192000f64.next_down(),
        "prepared rate extremum",
    )?;
    Ok(
        json!({"maximum":data,"rate_grid":grid,"epoch":field_offsets(),"proposed_temporary_locals":{"fields":stack.rows,"bytes":stack.end},"allocator_overhead":null,"measured_stack":null}),
    )
}
