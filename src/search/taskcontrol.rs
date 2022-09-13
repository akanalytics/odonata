use crate::domain::Game;
use crate::infra::component::{Component, State};
use anyhow::Context;
use anyhow::Result;
use fslock::LockFile;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use std::sync::Mutex;
use std::{fmt, io};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TaskControl<TTaskProgress> {
    pub log_dir: String,
    pub debug: bool,
    pub show_metrics_on_exit: bool,
    export_games: bool,

    #[serde(skip)]
    pub progress_callback: Option<Arc<Mutex<dyn Fn(&TTaskProgress) + Send + Sync>>>,

    #[serde(skip)]
    kill_switch: Arc<AtomicBool>,
}

impl<TTaskProgress> fmt::Display for TaskControl<TTaskProgress> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "debug            : {}", self.debug)?;
        writeln!(f, "log directory    : {}", self.log_dir)?;
        writeln!(f, "metrics on exit  : {}", self.show_metrics_on_exit)?;
        writeln!(f, "export games     : {}", self.export_games)?;
        writeln!(
            f,
            "kill switch      : {}",
            self.kill_switch.load(atomic::Ordering::Relaxed)
        )?;
        // writeln!(f, "has bn cancelled : {}", self.has_been_cancelled)?;
        writeln!(
            f,
            "progress_callback: {}",
            if self.progress_callback.is_some() {
                "set"
            } else {
                "not set"
            }
        )?;
        Ok(())
    }
}

impl<TTaskProgress> Component for TaskControl<TTaskProgress>
where
    TTaskProgress: Default,
{
    fn new_iter(&mut self) {}

    fn new_position(&mut self) {}

    fn new_game(&mut self) {}

    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame => {}
            SetPosition => {}
            StartSearch => {
                self.set_running();
            }
            EndSearch => {
                self.set_running();
            }
            StartDepthIteration(_) => {}
            Shutdown => {}
        }
    }
}

impl<TTaskProgress> TaskControl<TTaskProgress> {
    #[inline]
    pub fn cancel(&mut self) {
        self.kill_switch.store(true, atomic::Ordering::Relaxed);
    }

    pub fn export_game(&self, g: &Game) -> Result<()> {
        if self.export_games {
            let mut lock_file = LockFile::open(&format!("{}/game.lock", self.log_dir))?;
            lock_file.lock()?;
            let mut w = self.game_writer(g.game_id)?;
            writeln!(w, "# begin")?;
            g.export(&mut w)?;
            writeln!(w, "# end")?;
            lock_file.unlock()?;
        }
        Ok(())
    }

    pub fn game_writer(&self, _game_id: u32) -> Result<Box<dyn Write>> {
        if !self.export_games {
            Ok(Box::new(io::sink()))
        } else {
            // let filename = format!("{}/game-{:06}.csv", self.log_dir, game_id);
            // let _dt = Local::now().format("%Y-%m-%d-%H-%M-%S.%.3f").to_string();
            // let dt = "1"; //
            let filename = format!("{}/games.txt", self.log_dir);
            info!("Opening file {} for game export", filename);
            let f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&filename)
                .with_context(|| format!("Failed to open file {}", &filename))?;
            let writer = BufWriter::new(f);
            Ok(Box::new(writer))
        }
    }

    fn set_running(&mut self) {
        // self.has_been_cancelled = false;
        self.kill_switch.store(false, atomic::Ordering::Relaxed);
    }

    #[inline]
    pub fn is_cancelled(&mut self) -> bool {
        //if !self.has_been_cancelled {
        self.kill_switch.load(atomic::Ordering::Relaxed)
        //}
        // self.has_been_cancelled
    }

    pub fn invoke_callback(&self, data: &TTaskProgress) {
        if let Some(callback) = &self.progress_callback {
            let callback = callback.lock().unwrap();
            callback(data);
        }
    }

    pub fn register_callback(&mut self, callback: impl Fn(&TTaskProgress) + Send + Sync + 'static) {
        self.progress_callback = Some(Arc::new(Mutex::new(callback)));
    }
}
