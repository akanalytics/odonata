use std::fmt::Debug;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use once_cell::sync::Lazy;
use tracing::span::{Attributes, Id, Record};
use tracing::{callsite, subscriber, Metadata, Subscriber};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{filter, fmt, Registry};

use crate::infra::resources;

static CURRENT_LOGGING_SYSTEM: Mutex<LoggingSystem> = Mutex::new(LoggingSystem::empty());
static LOGGING_SUBSCRIBER: Lazy<Arc<MutSubscriber>> = Lazy::new(|| Arc::new(MutSubscriber::default()));

struct MutSubscriber {
    inner: Mutex<Box<dyn Subscriber + Send + Sync>>,
}

#[derive(Debug, Clone)]
#[must_use = "struct unused - invoke apply()"]
pub struct LoggingSystem {
    logfile: Option<PathBuf>, // None or empty disables the file logging
    levels:  String,
}

impl LoggingSystem {
    const fn empty() -> Self {
        Self {
            logfile: None,
            levels:  String::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let env = std::env::var("RUST_LOG").unwrap_or("error".to_string());
        Self::parse(&env)
    }

    pub fn init() -> Result<()> {
        subscriber::set_global_default(Arc::clone(&LOGGING_SUBSCRIBER))?;
        Self::from_env()?.apply()
    }

    /// examples:
    ///   ""  (turn off file logging, and stederr = error)
    ///   warn
    ///   uci=warn
    ///   logile.txt:warn
    ///   :warn  (turn off file logging)
    ///   /tmp/logfile.txt|uci=info,warn
    ///   /tmp/logfile.txt
    /// returns (log file, levels) from "odonata.log:uci=info,warn" for example
    pub fn parse(s: &str) -> Result<Self> {
        let Self { logfile, levels } = CURRENT_LOGGING_SYSTEM.lock().unwrap().clone();
        if s.is_empty() {
            return Ok(LoggingSystem {
                logfile: None,
                levels:  "error".to_string(),
            });
        }
        if !s.contains(['/', '.', ':']) {
            if let Ok(_parse_whole) = s.parse::<filter::Targets>() {
                return Ok(LoggingSystem {
                    logfile,
                    levels: s.to_string(),
                });
            }
        }
        if let Some((lf, lvls)) = s.split_once(':') {
            return Ok(LoggingSystem {
                logfile: Some(PathBuf::from(lf)),
                levels:  lvls.to_string(),
            });
        }
        if s.contains(|ch: char| ch == '=' || ch == ':' || ch == ' ' || !ch.is_ascii()) {
            anyhow::bail!("bad logging directives '{s}'");
        }

        Ok(LoggingSystem {
            logfile: Some(PathBuf::from(s)),
            levels,
        })
    }

    pub fn apply(self) -> Result<()> {
        let stderr_env_filter = self.levels.parse::<filter::Targets>()?;
        let file_layer = match &self.logfile {
            Some(lf) if !lf.as_os_str().is_empty() => {
                let file = RollingFileAppender::new(Rotation::NEVER, resources::workspace_dir(), lf.as_os_str());
                let layer = fmt::layer()
                    .with_ansi(false)
                    .with_target(false)
                    .with_writer(file)
                    .with_thread_names(true);
                Some(layer)
            }
            _ => None,
        };
        let stderr_layer = fmt::layer().with_writer(io::stderr).without_time().with_target(false);
        let new_subscriber = Registry::default()
            .with(file_layer)
            .with(stderr_layer)
            .with(stderr_env_filter);
        *LOGGING_SUBSCRIBER.inner.lock().unwrap() = Box::new(new_subscriber);
        callsite::rebuild_interest_cache();
        trace!("Logging change applied... {self:?}");
        *CURRENT_LOGGING_SYSTEM.lock().unwrap() = self;
        Ok(())
    }
}

impl Debug for MutSubscriber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MutSubscriber").finish()
    }
}

impl Default for MutSubscriber {
    fn default() -> Self {
        Self {
            inner: Mutex::new(Box::new(fmt::Subscriber::new())),
        }
    }
}

impl Subscriber for MutSubscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.inner.lock().unwrap().enabled(metadata)
    }

    fn new_span(&self, span: &Attributes<'_>) -> Id {
        self.inner.lock().unwrap().new_span(span)
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        self.inner.lock().unwrap().record(span, values)
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        self.inner.lock().unwrap().record_follows_from(span, follows)
    }

    fn event(&self, event: &tracing::Event<'_>) {
        self.inner.lock().unwrap().event(event)
    }

    fn enter(&self, span: &Id) {
        self.inner.lock().unwrap().enter(span)
    }

    fn exit(&self, span: &Id) {
        self.inner.lock().unwrap().exit(span)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::{fs, thread};

    use crate::trace::logger::LoggingSystem;

    #[test]
    #[ignore = "sets global logger so needs to run on its own"]
    fn test_logger() -> anyhow::Result<()> {
        // test invalid things
        assert_eq!(LoggingSystem::parse("uci = debug").is_err(), true);
        // assert_eq!(LoggingSystem::parse(":uci=invalid").is_err(), true);
        // assert_eq!(LoggingSystem::parse(":uci=").is_err(), true);

        // test levels parsed
        assert_eq!(LoggingSystem::parse("logfile.txt").unwrap().levels, "");
        assert_eq!(LoggingSystem::parse("").unwrap().levels, "error");
        assert_eq!(LoggingSystem::parse("off").unwrap().levels, "off");
        assert_eq!(LoggingSystem::parse("warn").unwrap().levels, "warn");
        assert_eq!(LoggingSystem::parse("uci=debug").unwrap().levels, "uci=debug");
        assert_eq!(LoggingSystem::parse(":uci=debug").unwrap().levels, "uci=debug");

        // test logfile
        assert_eq!(
            LoggingSystem::parse("logfile.txt")?.logfile,
            Some(PathBuf::from("logfile.txt"))
        );
        assert_eq!(LoggingSystem::parse("").unwrap().logfile, None);
        assert_eq!(LoggingSystem::parse(":").unwrap().logfile, Some(PathBuf::new()));
        assert_eq!(LoggingSystem::parse("warn").unwrap().logfile, None);
        assert_eq!(LoggingSystem::parse("uci=debug").unwrap().logfile, None);
        assert_eq!(
            LoggingSystem::parse(":uci=debug").unwrap().logfile,
            Some(PathBuf::new())
        );

        let mut tmp = std::env::temp_dir();
        tmp.push("odonata.log");
        let _ignore_file_missing = fs::remove_file(&tmp);

        error!("HIDDEN*** 01. error - pre-init");
        LoggingSystem::init()?;
        trace!("HIDDEN*** 02. trace");
        debug!("HIDDEN*** 03. debug");
        info!("HIDDEN*** 04. info");
        warn!("HIDDEN*** 05. warn");
        error!("SEEN 06. error");
        debug!(target: "uci", "HIDDEN 07. uci/debug");
        error!(target: "uci", "SEEN 08. uci/error");
        println!("06, 08");
        assert_eq!(tmp.exists(), false);

        LoggingSystem::parse("warn,uci=debug")?.apply()?;
        trace!("HIDDEN*** 10. trace");
        debug!("HIDDEN*** 11. debug");
        info!("HIDDEN*** 12. info");
        warn!("SEEN 13. warn");
        error!("SEEN 14. error");
        trace!(target: "uci", "HIDDEN 15. uci/trace");
        debug!(target: "uci", "SEEN 16. uci/debug");
        println!("13, 14, 16");
        assert_eq!(tmp.exists(), false);

        let spec = format!("{}:uci=trace,debug", tmp.display());
        LoggingSystem::parse(&spec)?.apply()?;
        debug!("SEEN 20. debug");
        warn!("SEEN 21. warn");
        warn!("SEEN 22. uci/debug");
        trace!(target: "uci", "SEEN 23. uci/trace");
        warn!("SEEN 24. warn");
        trace!("HIDDEN** 25. uci/trace");
        println!("SEEN 20, 21, 22, 23, 24");
        assert_eq!(tmp.exists(), true);

        let jh = thread::spawn(|| {
            info!("SEEN 26. info within thread");
            error!("SEEN 27. error within thread")
        });
        jh.join().expect("thread panic");

        LoggingSystem::from_env()?.apply()?;
        error!("SEEN 30. error");
        info!("HIDDEN*** 31. info");
        println!("LOG 26, 27, 30");

        println!("FILE\n{}", fs::read_to_string(&tmp)?);
        println!("LOG 20, 21 22, 23, 24, 26, 27, 30");

        fs::remove_file(&tmp)?;
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

// fn stderr_enabled<S>(meta: &Metadata<'_>, ctx: &Context<'_, S>) -> bool {
//     let ls = LoggingSystem::instance().unwrap();
//     let env = ls.stderr_env_log.lock().unwrap();
//     let enabled = Filter::<S>::enabled(&EnvFilter::try_new(env.as_str()).unwrap(), meta, ctx);
//     println!("stderr_enabled ...{env}={enabled}");
//     enabled
// }

// fn std_callsite_enabled<S>(_meta: &'static Metadata<'static>) -> Interest {
//     let Ok(_ls) = LoggingSystem::instance() else {
//         return Interest::never();
//     };
//     Interest::sometimes()
//     // Filter::<S>::callsite_enabled(&EnvFilter::new(ls.stderr_env_log.as_str()), meta)
// }

// fn make_file_writer() -> File {
//     let logfile = if let Ok(ls) = LoggingSystem::instance() {
//         PathBuf::from(&ls.logfile)
//     } else {
//         LoggingSystem::default().logfile
//     };
//     let logfile = resources::workspace_dir().join(logfile);
//     OpenOptions::new()
//         .create(true)
//         .append(true)
//         .open(&logfile)
//         .unwrap_or_else(|_| panic!("unable to create/append logfile {logfile:?}"))
// }
// *sub.subscriber.lock().unwrap() = subscriber;

// layer 1 is the file writer
// let layer1 = fmt::Layer::default()
//     .compact()
//     .with_ansi(true)
//     .without_time()
//     .with_line_number(false)
//     .with_writer(make_file_writer)
//     .with_filter(DynFilterFn::new(stderr_enabled));
// .with_filter(DynFilterFn::new(|metadata, cx| {
//     if metadata.target().contains("uci") {
//         return true;
//     }
//     false
// }));

// let layer2 = fmt::Layer::default()
//     .compact()
//     .with_ansi(true)
//     .with_writer(io::stderr)
//     .with_line_number(false)
//     .with_filter(DynFilterFn::new(stderr_enabled));

// Registry::default().with(layer1).with(layer2).init();
// println!("{:?}", sub.subscriber.lock().unwrap());

// let env_filter: EnvFilter = EnvFilter::from_default_env();
// let subscriber = Registry::default().with(env_filter);
// sub.set_subscriber(MutSubscriber { subscriber });
