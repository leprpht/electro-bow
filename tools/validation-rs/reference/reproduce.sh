#!/bin/sh
# Historical comparator, run from the repository root on arm64 macOS after building the Release bundle.
set -eu
root=$(pwd -P)
ref=${ELECTROBOW_MIGRATION_REFERENCE:-"$root/build/validation-migration/reference"}
py=${ELECTROBOW_REFERENCE_PYTHON:-"$root/build/spec-reconciliation/py/bin/python"}
tools="$root/tools/validation-rs/reference"
entry=fcc272f8ec7a0472917c1e787edbd2918dcc3ae0
mkdir -p "$ref"
if [ ! -d "$ref/checkout" ]; then git worktree add --detach "$ref/checkout" "$entry"; fi
cd "$ref/checkout"
test "$(git rev-parse HEAD)" = "$entry"
test -z "$(git status --porcelain)"
"$py" -c 'import sys; assert sys.version_info[:3] == (3, 9, 6)'
"$py" -m pip freeze > "$ref/python-freeze.txt"
for package in $(cat "$tools/requirements-macos-arm64.txt"); do grep -Fxiq "$package" "$ref/python-freeze.txt"; done
export OPENBLAS_NUM_THREADS=1 VECLIB_MAXIMUM_THREADS=1 PYTHONPATH=tools/validation
export ELECTROBOW_O2_OUT=build/o2 ELECTROBOW_I1_OUT=build/i1
mkdir -p build/spec-signoff
if [ ! -e build/ElectroBow_artefacts ]; then ln -s "$root/build/ElectroBow_artefacts" build/ElectroBow_artefacts; fi
measure(){
    label=$1
    shift
    if /usr/bin/time -lp "$@" >"$ref/$label.stdout" 2>"$ref/$label.stderr"; then code=0; else code=$?; fi
    echo "$label $code" >> "$ref/reproduction-exits.txt"
    test "$code" = 0
}
measure fixtures shasum -a 256 -c tools/validation/fixtures/acoustic/SHA256SUMS
measure contact-build rustc --edition 2024 -O tools/validation/contact_replay.rs -o build/contact
measure contact "$py" tools/validation/contact_contract.py --rust build/contact
measure ring "$py" tools/validation/ring_oracle.py
measure guide "$py" tools/validation/guide_qualification.py
measure observations "$py" tools/validation/acoustic_observation.py
measure acoustic-identity "$py" tools/validation/acoustic_identity.py
measure o1 "$py" tools/validation/o1_qualification.py
measure o1-state "$py" tools/validation/o1_state_parity.py
measure electrical "$py" tools/validation/o2_qualification.py
measure publication "$py" tools/validation/o2_publication_checks.py
measure identity "$py" tools/validation/i1_qualification.py
for suite in contact_mutations ring_mutations guide_mutations guide_negative_controls o1_mutations o2_mutations; do
    measure "$suite" "$py" "tools/validation/$suite.py"
done
measure guide-stress build/stage-03/guide-render --stress
measure plugin "$py" tools/validation/plugin_baseline.py
measure pitch-build rustc --edition 2024 -O tools/validation/pitch_candidates.rs -o build/spec-signoff/pitch_candidates
measure pitch "$py" tools/validation/pitch_study.py
measure pitch-results "$py" tools/validation/check_pitch_results.py
measure freeze "$py" "$tools/freeze.py" "$ref"
measure freeze-bank "$py" "$tools/freeze_bank.py" "$ref/bank"
measure freeze-guide "$py" "$tools/freeze_guide.py" build/stage-03 "$ref"
measure freeze-acoustic "$py" "$tools/freeze_acoustic.py" build/o2/results.json "$ref/acoustic-result.json"
# Compile and execute the Review 8.2 defective producer in a new private source directory.
variant=$(mktemp -d "$ref/checkout/build/half-product-tools-XXXXXX")
cp -R tools/validation/. "$variant/"
"$py" - "$variant/i1_joined.py" <<'PY'
from pathlib import Path
import sys
p = Path(sys.argv[1])
text = p.read_text()
before = "product = float(e) * (float(signed_gain) * float(wet))"
assert text.count(before) == 1
p.write_text(text.replace(before, before + " * 0.5"))
PY
measure half-product env PYTHONPATH="$variant" ELECTROBOW_I1_OUT=build/half-product "$py" "$variant/i1_qualification.py"
measure freeze-joined "$py" "$tools/freeze_joined.py" "$ref"
