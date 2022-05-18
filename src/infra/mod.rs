pub mod component;
pub mod config;
pub mod resources;
pub mod serde;
pub mod tracer;
pub mod version;

#[cfg(test)]
pub mod profiler;


// from iai/bencher/criterion etc - the "standard" black_box def
pub fn black_box<T>(dummy: T) -> T {
    unsafe {
        let ret = std::ptr::read_volatile(&dummy);
        std::mem::forget(dummy);
        ret
    }
}
