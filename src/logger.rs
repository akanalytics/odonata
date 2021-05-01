use log::{Record, Level, Metadata};

struct SimpleLogger;

impl log::Log for SimpleLogger {

    #[inline]
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    #[inline]
    fn log(&self, record: &Record) {

        if self.enabled(record.metadata()) {
            eprintln!("{} - {}", record.level(), record.args());
        }
    }

    #[inline]
    fn flush(&self) {}
}

// macro_rules! debug {
//     ($($arg:tt)*) => (if cfg!(debug_assertions) { log!(::log::DEBUG, $($arg)*) })
// }

#[macro_export]
macro_rules! log_debug {
    (target: $target:expr, $($arg:tt)*) => (
        {crate::logger::init(); log!(target: $target, log::Level::Debug, $($arg)*)};
    );
    ($($arg:tt)*) => (
        {crate::logger::init(); log!(log::Level::Debug, $($arg)*)};
    )
}



// macro_rules! log {
//     (target: $target:expr, $lvl:expr, $($arg:tt)+) => ({
//         let lvl = $lvl;
//             andy_log(
//                 __log_format_args!($($arg)+),
//                 lvl,
//                 &($target, __log_module_path!(), __log_file!(), __log_line!()),
//             );
//     });
//     ($lvl:expr, $($arg:tt)+) => (log!(target: __log_module_path!(), $lvl, $($arg)+))
// }



// // WARNING: this is not part of the crate's public API and is subject to change at any time
// #[doc(hidden)]
// pub fn andy_log(
//     args: fmt::Arguments,
//     level: Level,
//     &(target, module_path, file, line): &(&str, &'static str, &'static str, u32),
// ) {
//     println!("Hello andy");
//     init();
//     logger().log(
//         &Record::builder()
//             .args(args)
//             .level(level)
//             .target(target)
//             .module_path_static(Some(module_path))
//             .file_static(Some(file))
//             .line(Some(line))
//             .build(),
//     );
// }



// #[cfg(not(feature = "slim"))]
// macro_rules! debug {
//     ($($arg: tt)*) => { debug!($($arg)*) }
// }

// #[cfg(feature = "slim")]
// macro_rules! debug {
//     ($($arg: tt)*) => { }
//}

use log::{LevelFilter};

static LOGGER: SimpleLogger = SimpleLogger;

// pub fn init() -> Result<(), SetLoggerError> {
//     log::set_logger(&LOGGER)
//         .map(|()| log::set_max_level(LevelFilter::Debug))
// }

#[inline]
pub fn init() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Debug)).unwrap_or(());
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_logger() {
        // println!("Printed!");
        // log!("debug: Hellow World!");
        debug!("debug: Hellow World!");
        info!("info: Hellow World!");
        error!("error: Hellow World!");
    }
}