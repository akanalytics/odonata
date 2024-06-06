use std::fmt;
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;

use odonata_base::domain::info::Info;
use odonata_base::infra::component::{Component, State};
use odonata_base::prelude::*;
use serde::{Deserialize, Serialize};

use super::algo::Callback;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Controller {
    pub show_refutations: bool,
    pub analyse_mode:     bool, // tries to find full PV etc
    pub multi_pv:         usize,

    #[serde(skip)]
    pub progress_callback: Callback,

    #[serde(skip)]
    kill_switch: Arc<AtomicBool>,
}

impl Default for Controller {
    fn default() -> Self {
        Self {
            show_refutations:  false,
            analyse_mode:      false,
            multi_pv:          1,
            progress_callback: Callback::default(),
            kill_switch:       Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Configurable for Controller {
    fn set(&mut self, p: Param) -> Result<bool> {
        self.show_refutations.set(p.get("show_refutations"))?;
        self.analyse_mode.set(p.get("analyse_mode"))?;
        self.multi_pv.set(p.get("multi_pv"))?;
        Ok(p.is_modified())
    }
}

impl fmt::Display for Controller {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "kill switch      : {}",
            self.kill_switch.load(atomic::Ordering::SeqCst)
        )?;
        Ok(())
    }
}

impl Component for Controller {
    fn new_iter(&mut self) {}

    fn new_position(&mut self) {}

    fn new_game(&mut self) {
        self.kill_switch.store(false, atomic::Ordering::SeqCst);
    }

    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame => self.new_game(),
            SetPosition => {}
            StartSearch => {}
            EndSearch => {}
            StartDepthIteration(_) => {}
            Shutdown => {}
        }
    }
}

impl Controller {
    #[inline]
    pub fn cancel(&mut self) {
        self.kill_switch.store(true, atomic::Ordering::SeqCst);
    }

    pub fn set_running(&mut self) {
        // self.has_been_cancelled = false;
        self.kill_switch.store(false, atomic::Ordering::SeqCst);
    }

    #[inline]
    pub fn is_cancelled(&mut self) -> bool {
        // if !self.has_been_cancelled {
        self.kill_switch.load(atomic::Ordering::SeqCst)
        //}
        // self.has_been_cancelled
    }

    pub fn invoke_callback(&self, info: &Info) {
        trace!("callback with {info}");
        self.progress_callback.0(info);
    }

    pub fn register_callback(&mut self, callback: impl Fn(&Info) + Send + Sync + 'static) {
        self.progress_callback = Callback(Arc::new(callback));
    }
}
