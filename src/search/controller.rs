use crate::{
    domain::info::Info,
    infra::component::{Component, State},
};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    sync::{
        atomic::{self, AtomicBool},
        Arc, Mutex,
    },
};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Controller {
    pub show_refutations: bool,
    pub analyse_mode:     bool, // tries to find full PV etc

    #[serde(skip)]
    pub progress_callback: Option<Arc<Mutex<dyn Fn(&Info) + Send + Sync>>>,

    #[serde(skip)]
    kill_switch: Arc<AtomicBool>,
}

impl fmt::Display for Controller {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(
            f,
            "kill switch      : {}",
            self.kill_switch.load(atomic::Ordering::SeqCst)
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

impl Component for Controller {
    fn new_iter(&mut self) {}

    fn new_position(&mut self) {}

    fn new_game(&mut self) {}

    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame => {}
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

    pub fn invoke_callback(&self, data: &Info) {
        trace!("callback with {data}");
        if let Some(callback) = &self.progress_callback {
            let callback = callback.lock().unwrap();
            callback(data);
        }
    }

    pub fn register_callback(&mut self, callback: impl Fn(&Info) + Send + Sync + 'static) {
        self.progress_callback = Some(Arc::new(Mutex::new(callback)));
    }
}
