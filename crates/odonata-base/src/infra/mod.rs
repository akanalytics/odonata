pub mod component;
pub mod config;
pub mod metric;
pub mod resources;
pub mod serde;
pub mod testing;
pub mod tracer;
pub mod utils;
pub mod value;
pub mod lockless_hashmap;

#[cfg(target_os = "linux")]
pub mod profiler;

// from iai/bencher/criterion etc - the "standard" black_box def
// #[inline(always)]
// pub fn black_box<T>(dummy: T) -> T {
//     unsafe {
//         let ret = std::ptr::read_volatile(&dummy);
//         std::mem::forget(dummy);
//         ret
//     }
// }
