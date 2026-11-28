//! The reviewed offline C++ candidate. The independent full-window oracle never calls these functions.
use crate::{Result, ensure, process};
use libloading::Library;
use std::{ffi::c_void, path::Path, process::Command};

type Init = unsafe extern "C" fn(*mut c_void);
type Run = unsafe extern "C" fn(*mut c_void, f64, *const f64, *mut f64, i32, i32, *mut i32) -> i32;
type Stats = unsafe extern "C" fn(*mut c_void, *mut f64, i32);

pub struct Kernel {
    _library: Library,
    init: Init,
    run: Run,
    stats: Stats,
}
impl Kernel {
    pub fn build(out: &Path) -> Result<Self> {
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
                    "tools/validation/i1_stream.cpp",
                    "-o",
                ])
                .arg(&path),
        )?;
        // SAFETY: this library was built above from the fixed, non-shipping C ABI.
        // Its four exported signatures and object size are declared at the end of i1_stream.cpp.
        unsafe {
            let lib = Library::new(path)?;
            let size = lib.get::<unsafe extern "C" fn() -> usize>(b"epoch_size")?();
            ensure(size == 3880, "offline epoch ABI size")?;
            Ok(Self {
                init: *lib.get(b"epoch_init")?,
                run: *lib.get(b"epoch_run")?,
                stats: *lib.get(b"epoch_stats")?,
                _library: lib,
            })
        }
    }
    pub fn epoch(&self, fs: f64) -> Epoch<'_> {
        let mut state = Box::new([0u64; 485]);
        // SAFETY: 485 aligned eight-byte words cover exactly the checked 3880-byte Epoch.
        unsafe { (self.init)(state.as_mut_ptr().cast()) };
        Epoch {
            state,
            kernel: self,
            fs,
        }
    }
}
pub struct Epoch<'a> {
    state: Box<[u64; 485]>,
    kernel: &'a Kernel,
    fs: f64,
}
impl Epoch<'_> {
    pub fn run(&mut self, input: &[[f64; 20]], output: &mut [[f64; 20]], probe: i32) -> Result<()> {
        ensure(input.len() == output.len(), "epoch buffer length")?;
        let n = i32::try_from(input.len())?;
        let mut done = 0;
        // SAFETY: unique live state, disjoint row-major buffers, exactly n rows of 20 words.
        let code = unsafe {
            (self.kernel.run)(
                self.state.as_mut_ptr().cast(),
                self.fs,
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
        self.state[3808 / 8] = value.to_bits();
    }
    pub fn stats(&mut self, retired: bool) -> [f64; 123] {
        let mut words = [0.; 123];
        // SAFETY: the C++ static assertion fixes each Stats block at 984 bytes.
        unsafe {
            (self.kernel.stats)(
                self.state.as_mut_ptr().cast(),
                words.as_mut_ptr(),
                i32::from(retired),
            )
        };
        words
    }
}
