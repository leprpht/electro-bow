//! The reviewed offline C++ candidate. The independent full-window oracle never calls these functions.
use crate::{Result, ensure, process};
use libloading::Library;
use std::{ffi::c_void, path::Path, process::Command};

type Init = unsafe extern "C" fn(*mut c_void);
type Run = unsafe extern "C" fn(*mut c_void, f64, *const f64, *mut f64, i32, i32, *mut i32) -> i32;
type Stats = unsafe extern "C" fn(*mut c_void, *mut f64, i32);

#[derive(Debug)]
pub(super) struct EpochFault(pub i32, pub usize);
impl std::fmt::Display for EpochFault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "epoch fault site={} offset={}", self.0, self.1)
    }
}
impl std::error::Error for EpochFault {}
pub enum Kernel {
    Historical(Native),
    Corrected,
}
pub struct Native {
    _library: Library,
    init: Init,
    run: Run,
    stats: Stats,
}
impl Kernel {
    pub fn build(out: &Path) -> Result<Self> {
        Self::from_source(out, Path::new("tools/validation/i1_stream.cpp"))
    }
    pub(super) fn from_source(out: &Path, source: &Path) -> Result<Self> {
        let path = out
            .canonicalize()?
            .join(format!("epoch{}", std::env::consts::DLL_SUFFIX));
        process::check(
            Command::new("clang++")
                .args([
                    "-std=c++17",
                    "-O3",
                    "-ffp-contract=off",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-shared",
                    "-fPIC",
                ])
                .arg(source)
                .arg("-o")
                .arg(&path),
        )?;
        // SAFETY: this library was built above from the fixed, non-shipping C ABI.
        // Its four exported signatures and object size are declared at the end of i1_stream.cpp.
        unsafe {
            let lib = Library::new(path)?;
            let size = lib.get::<unsafe extern "C" fn() -> usize>(b"epoch_size")?();
            ensure(size == 3880, "offline epoch ABI size")?;
            Ok(Self::Historical(Native {
                init: *lib.get(b"epoch_init")?,
                run: *lib.get(b"epoch_run")?,
                stats: *lib.get(b"epoch_stats")?,
                _library: lib,
            }))
        }
    }
    pub fn epoch(&self, fs: f64) -> Epoch<'_> {
        let Self::Historical(kernel) = self else {
            return Epoch::Rust(Box::new(super::candidate::Candidate::new(fs)));
        };
        let mut state = Box::new([0u64; 485]);
        // SAFETY: 485 aligned eight-byte words cover exactly the checked 3880-byte Epoch.
        unsafe { (kernel.init)(state.as_mut_ptr().cast()) };
        Epoch::Native(NativeEpoch { state, kernel, fs })
    }
}
pub enum Epoch<'a> {
    Native(NativeEpoch<'a>),
    Rust(Box<super::candidate::Candidate>),
}
pub struct NativeEpoch<'a> {
    state: Box<[u64; 485]>,
    kernel: &'a Native,
    fs: f64,
}
impl Epoch<'_> {
    pub fn run(&mut self, input: &[[f64; 20]], output: &mut [[f64; 20]], probe: i32) -> Result<()> {
        ensure(input.len() == output.len(), "epoch buffer length")?;
        let native = match self {
            Self::Native(native) => native,
            Self::Rust(state) => {
                for (i, (frame, row)) in input.iter().zip(output).enumerate() {
                    state
                        .tick(frame, row, probe)
                        .map_err(|site| EpochFault(site, i))?;
                }
                return Ok(());
            }
        };
        let n = i32::try_from(input.len())?;
        let mut done = 0;
        // SAFETY: unique live state, disjoint row-major buffers, exactly n rows of 20 words.
        let code = unsafe {
            (native.kernel.run)(
                native.state.as_mut_ptr().cast(),
                native.fs,
                input.as_ptr().cast(),
                output.as_mut_ptr().cast(),
                n,
                probe,
                &mut done,
            )
        };
        ensure(
            code == 0 && done == n,
            &format!("epoch fault site={code} offset={done}"),
        )
    }
    pub fn inject_stored_phase(&mut self, value: f64) {
        // Existing Review 8.2 probe offset in the size-checked offline C++ object.
        match self {
            Self::Native(native) => native.state[3808 / 8] = value.to_bits(),
            Self::Rust(state) => state.phase = value,
        }
    }
    pub fn stats(&mut self, retired: bool) -> [f64; 123] {
        let native = match self {
            Self::Native(native) => native,
            Self::Rust(state) => return state.stats(retired),
        };
        let mut words = [0.; 123];
        // SAFETY: the C++ static assertion fixes each Stats block at 984 bytes.
        unsafe {
            (native.kernel.stats)(
                native.state.as_mut_ptr().cast(),
                words.as_mut_ptr(),
                i32::from(retired),
            )
        };
        words
    }
}
