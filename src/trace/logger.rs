// use env_logger::{Builder, Env};
// use once_cell::sync::Lazy;
// use log::Log;
// use tracing::Dispatch;
// use tracing::metadata::LevelFilter;
// use tracing_subscriber::{Registry, fmt};
// use tracing_subscriber::fmt::{format, layer};
use std::io;
use tracing_subscriber::{EnvFilter, FmtSubscriber}; // , reload::Handle};
                                                    // use tracing::subscriber::Subscriber;
                                                    //use tracing_subscriber::layer::Layer;
                                                    // use tracing_subscriber::layer::Layered;
                                                    //use tracing_subscriber::fmt::Formatter;

// the logging macros are a bit crufty. to use them in unit tests we check initialization on every use.
// this means redefining the debug!/info!/warn! etc macros.
// ideally i'd like to use crate::logger::LogInit in the macro to avoid and includes of "impl" details but
// this doesnt seem to work for benchmark executable.
// Logging is also slow(ish), so not in any really tight loops
// Benchmarking logging: Warming up for 3.0000 sInitilaized logging
// logging                 time:   [1.8780 ns 1.8884 ns 1.8994 ns]

pub struct LogInit;

// type ReloadHandle = Handle<EnvFilter, Formatter<format::DefaultFields,format::Format<format::Full>,fn() -> io::Stderr>>;

// struct ReloadHandle {
//     handle: Handle<EnvFilter, Formatter>,
// }

pub fn init_logging() {
    let builder = FmtSubscriber::builder()
        .with_ansi(true)
        .with_writer(io::stderr)
        .with_env_filter(EnvFilter::from_default_env())
        .with_filter_reloading();

    //    let reload_handle = builder.reload_handle();

    builder.init();

    // let dispatch = Dispatch::new(builder.finish());

    // let t = builder.downcast_ref::<ReloadHandle>();

    // reload_handle
}

// let rh = ReloadHandle {
//     handle: reload_handle,
// };

// pub type MyFormatter<
//     N = format::DefaultFields,
//     E = format::Format<format::Full>,
//     W = fn() -> io::Stderr,
// > = fmt::layer::Layered<fmt_layer::Layer<Registry, N, E, W>, Registry>;

// fn subscriber_downcasts_to_parts() {
//     let subscriber = Subscriber::new();
//     let dispatch = Dispatch::new(subscriber);
//     assert!(dispatch.downcast_ref::<format::DefaultFields>().is_some());
//     assert!(dispatch.downcast_ref::<LevelFilter>().is_some());
//     assert!(dispatch.downcast_ref::<format::Format>().is_some())

// impl LogInit {
//     #[inline]
//     pub fn bootstrap() -> Self {
//         let mut builder = Builder::from_env(Env::default().default_filter_or("warn"));
//         builder.init();
//         LogInit {}
//     }

//     #[inline]
//     pub fn dummy_init() {
//         // keeps compiler warnings quiet when feature disabled
//     }

//     #[inline]
//     pub fn check_init() {
//         Lazy::force(&LOGGER);
//     }
// }

// static LOGGER: Lazy<LogInit> = Lazy::new(|| LogInit::bootstrap());

// #[macro_export]
// macro_rules! trace {
//     ($($arg:tt)*) => (
//         {
//             #[cfg(not(feature="remove_logging"))]
//             LogInit::check_init();

//             #[cfg(feature="remove_logging")]
//             LogInit::dummy_init();

//             log::log!(log::Level::Trace, $($arg)*)
//         };
//     )
// }

// #[macro_export]
// macro_rules! debug {
//     ($($arg:tt)*) => (
//         {
//             #[cfg(not(feature="remove_logging"))]
//             LogInit::check_init();

//             #[cfg(feature="remove_logging")]
//             LogInit::dummy_init();

//             log::log!(log::Level::Debug, $($arg)*)
//         };
//     )
// }

// #[macro_export]
// macro_rules! info {
//     ($($arg:tt)*) => (
//         {
//             LogInit::check_init();
//             log::log!(log::Level::Info, $($arg)*)
//         };
//     )
// }

// #[macro_export]
// macro_rules! warn {
//     ($($arg:tt)*) => (
//         {
//             LogInit::check_init();
//             log::log!(log::Level::Warn, $($arg)*)
//         };
//     )
// }

// #[macro_export]
// macro_rules! error {
//     ($($arg:tt)*) => (
//         {
//             LogInit::check_init();
//             log::log!(log::Level::Error, $($arg)*)
//         };
//     )
// }

#[cfg(test)]
mod tests {

    #[test]
    fn test_logger() {
        // log::set_max_level(log::LevelFilter::Info);
        trace!("trace: Hellow world!");
        debug!("debug: Hellow world!");
        info!("info: Hellow world!");
        // warn!("warn: Hellow world!");
        // error!("error: Hellow world!");
        // log::set_max_level(log::LevelFilter::Trace);
        debug!("debug: Debug enabled!");
    }
}
