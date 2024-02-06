// use env_logger::{Builder, Env};
// use once_cell::sync::Lazy;
// use log::Log;
// use tracing::Dispatch;
// use tracing::metadata::LevelFilter;
// use tracing_subscriber::{Registry, fmt};
// use tracing_subscriber::fmt::{format, layer};
// use std::io;
// use tracing_subscriber::{EnvFilter, FmtSubscriber}; // , reload::Handle};
// use tracing::subscriber::Subscriber;
// use tracing_subscriber::layer::Layer;
// use tracing_subscriber::layer::Layered;
// use tracing_subscriber::fmt::Formatter;

// the logging macros are a bit crufty. to use them in unit tests we check initialization on every use.
// this means redefining the debug!/info!/warn! etc macros.
// ideally i'd like to use crate::logger::LogInit in the macro to avoid and includes of "impl" details but
// this doesnt seem to work for benchmark executable.
// Logging is also slow(ish), so not in any really tight loops
// Benchmarking logging: Warming up for 3.0000 sInitilaized logging
// logging                 time:   [1.8780 ns 1.8884 ns 1.8994 ns]

use std::{
    io::{stderr, Write},
    sync::{Arc, Mutex},
};

use flexi_logger::{
    colored_default_format,
    writers::{FileLogWriter, LogWriter},
    AdaptiveFormat, DeferredNow, FileSpec, LevelFilter, LogSpecBuilder, LogSpecification, Logger,
    LoggerHandle, Record, WriteMode,
};

// inspired by https://github.com/emabee/flexi_logger/issues/124
struct OptLogWriterDecorator {
    log_file: Arc<Mutex<Option<Box<dyn LogWriter>>>>,
}

impl LogWriter for OptLogWriterDecorator {
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        let mut lf = self.log_file.lock().unwrap();
        if let Some(log_file) = &mut *lf {
            log_file.write(now, record)?;
        } else {
            colored_default_format(&mut stderr(), now, record)?;
            writeln!(stderr())?;
            stderr().flush()?;
            // println!(
            //     "{}:{} -- {}",
            //     record.level(),
            //     record.target(),
            //     record.args()
            // );
        }
        Ok(())
    }

    fn flush(&self) -> std::io::Result<()> {
        let mut lf = self.log_file.lock().unwrap();
        if let Some(ref log_file) = &mut *lf {
            log_file.flush()?;
        }
        Ok(())
    }
}

use once_cell::sync::OnceCell;

static LOGGING_SYSTEM: OnceCell<LoggingSystem> = OnceCell::new();

pub struct LoggingSystem {
    handle:   LoggerHandle,
    log_spec: LogSpecification,
    log_file: Arc<Mutex<Option<Box<dyn LogWriter>>>>,
}

pub fn plain(w: &mut dyn Write, _: &mut DeferredNow, r: &Record) -> Result<(), std::io::Error> {
    write!(w, "{}", r.args())
}

impl LoggingSystem {
    pub fn instance() -> anyhow::Result<&'static Self> {
        LOGGING_SYSTEM
            .get_or_try_init(|| -> anyhow::Result<LoggingSystem> { LoggingSystem::new() })
    }

    fn new() -> anyhow::Result<Self> {
        let log_file = Arc::new(Mutex::new(None));
        let writer = Box::new(OptLogWriterDecorator {
            log_file: Arc::clone(&log_file),
        });
        // let file_spec = FileSpec::default();
        // let l = Logger::try_with_env_or_str("info")?
        //     .log_to_file(file_spec)
        //     .build()?;

        // uses RUST_LOG env var
        let log_spec = LogSpecification::env_or_parse("warn")?;
        let handle = Logger::with(log_spec.clone())
            .log_to_writer(writer)
            .adaptive_format_for_stderr(AdaptiveFormat::Default)
            // .write_mode(WriteMode::SupportCapture)
            // .write_mode(WriteMode::Direct)
            .write_mode(WriteMode::BufferAndFlush)
            // .duplicate_to_stderr(Duplicate::All)
            .start()?;
        info!("Logging enabled");
        debug!("inititial {log_spec:?}");
        Ok(LoggingSystem {
            handle,
            log_file,
            log_spec,
        })
    }

    pub fn init() -> anyhow::Result<()> {
        let _ = LoggingSystem::instance()?;
        Ok(())
    }

    /// s containing {pid} or {PID} is replaced with process id
    /// if s is empty, then a fallback path/filename is used
    /// if s ends with "." or "/" then directory assumed, fallback is appended as filename
    pub fn set_log_filename(&self, s: &str, fallback: &str) -> anyhow::Result<()> {
        if s.is_empty() {
            info!("turning file logging off");
            let builder = LogSpecBuilder::from_module_filters(self.log_spec.module_filters());
            let spec = builder.build_with_textfilter(self.log_spec.text_filter().cloned());
            debug!("file using new log spec: {spec:?}");
            self.handle.set_new_spec(spec);
            *self.log_file.lock().unwrap() = None;
            return Ok(());
        }

        let file_spec = if s.ends_with(['/', '.']) {
            info!("turning file logging on with directory '{s}' and basename '{fallback}'");
            FileSpec::default()
                .directory(s)
                .basename("odonata")
                .suppress_timestamp()
        }
        // we split on the last "." delimiter, allowing . embedded in the filename
        else if let Some((name, suffix)) = s.rsplit_once('.') {
            let pid = format!("{}", std::process::id());
            let name = name.replace("{pid}", &pid);
            let name = name.replace("{PID}", &pid);
            info!("turning file logging on with basename '{name}' and suffix '{suffix}'");
            FileSpec::default()
                .basename(name)
                .suffix(suffix)
                .suppress_timestamp()
        } else {
            info!("turning file logging on with basename '{s}'");
            FileSpec::default().basename(s).suppress_timestamp()
        };
        info!(
            "using '{name}' for file logging",
            name = file_spec.as_pathbuf(None).display()
        );
        let writer = Box::new(
            FileLogWriter::builder(file_spec)
                .format(plain)
                .try_build()?,
        );
        *self.log_file.lock().unwrap() = Some(writer);

        // turn on uci logging
        let mut builder = LogSpecBuilder::from_module_filters(self.log_spec.module_filters());
        builder.module("uci", LevelFilter::Debug);
        builder.module("eng", LevelFilter::Debug);
        let spec = builder.build_with_textfilter(self.log_spec.text_filter().cloned());
        debug!("file using new log spec: {spec:?}");
        self.handle.set_new_spec(spec);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::trace::logger::LoggingSystem;

    #[test]
    #[ignore = "sets global logger so needs to run on its own"]
    fn test_logger() -> anyhow::Result<()> {
        // log::set_max_level(log::LevelFilter::Info);
        error!("error: Not logged");
        LoggingSystem::init()?;
        trace!("trace: Hello world! NOT logged");
        debug!("debug: Hello world! NOT logged");
        info!("info: Hello world! NOT logged");
        warn!("warn: Hello world!");
        error!("error: Hello world!");
        debug!(target: "uci", "debug: on uci should NOT get logged");
        // log::set_max_level(log::LevelFilter::Trace);
        debug!("debug: Debug enabled!");
        LoggingSystem::instance()?.set_log_filename("/tmp/", "test_logging3")?;
        warn!("warn: Hello world logged to file3!");
        warn!(target: "ucimsg", "warn: ucimsg target!");
        trace!(target: "ucimsg", "trace: ucimsg target!");
        warn!(target: "", "warn: empty target!");
        LoggingSystem::instance()?.set_log_filename("/tmp/", "odonata2")?;
        warn!(target: "uci", "warn: on uci should get logged");
        debug!(target: "uci", "debug: on uci should get logged");
        Ok(())
    }
}

// log4rs
// let stderr = ConsoleAppender::builder().target(Target::Stderr).build();
// use log4rs::{
//     append::{
//         console::{ConsoleAppender, Target},
//         file::FileAppender,
//     },
//     config::{Appender, Root},
//     filter::threshold::ThresholdFilter,
//     Config,
// };

// // let logfile = FileAppender::builder()
// //     // Pattern: https://docs.rs/log4rs/*/log4rs/encode/pattern/index.html
// //     // .encoder(Box::new(PatternEncoder::new("{d} - {l} - {m}\n")))
// //     .build(file_path)
// //     .unwrap();

// let config = Config::builder()
//     // .appender(Appender::builder().build("logfile", Box::new(logfile)))
//     .appender(
//         Appender::builder()
//             .filter(Box::new(ThresholdFilter::new(LevelFilter::Info)))
//             .build("stderr", Box::new(stderr)),
//     )
//     .build(
//         Root::builder()
//             // .appender("logfile")
//             .appender("stderr")
//             .build(LevelFilter::Trace),
//     )?;
// let _handle = log4rs::init_config(config);
// Ok(())

// log4rs::init_config(config)?;
// flexi_logger
//     let handle = Logger::try_with_env_or_str("info")?
//         .adaptive_format_for_stderr(AdaptiveFormat::Default)
//         .start()?;
//     *LOG_HANDLE.write() = Some(handle);
//     Ok(())
// }

// pub fn log_to_file(filename: &str) -> anyhow::Result<()> {
//     let new_spec = LogSpecification::builder();
//     if let Some(lh) = *LOG_HANDLE.write() {
//         lh.set_new_spec(new_spec);
//     }
//     Ok(())
// }
// env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
//     .format_module_path(false)
//     .format_target(false)
//     .format_timestamp(None)
//     .init();

// type ReloadHandle = Handle<EnvFilter, Formatter<format::DefaultFields,format::Format<format::Full>,fn() -> io::Stderr>>;

// struct ReloadHandle {
//     handle: Handle<EnvFilter, Formatter>,
// }

// pub fn init_logging() {
//     let builder = FmtSubscriber::builder()
//         .with_ansi(true)
//         .with_writer(io::stderr)
//         .with_env_filter(EnvFilter::from_default_env())
//         .with_filter_reloading();

//     //    let reload_handle = builder.reload_handle();

//     builder.init();

//     // let dispatch = Dispatch::new(builder.finish());

//     // let t = builder.downcast_ref::<ReloadHandle>();

//     // reload_handle
// }

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
