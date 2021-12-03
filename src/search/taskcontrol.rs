use std::fmt;
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone, Default)]
pub struct TaskControl<TTaskProgress> {
    pub progress_callback: Option<Arc<Mutex<dyn Fn(&TTaskProgress) + Send + Sync>>>,
    kill_switch: Arc<AtomicBool>,
    // has_been_cancelled: bool,
}

impl<TTaskProgress> fmt::Display for TaskControl<TTaskProgress> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "kill switch      : {}", self.kill_switch.load(atomic::Ordering::Relaxed))?;
        // writeln!(f, "has bn cancelled : {}", self.has_been_cancelled)?;
        writeln!(
            f,
            "progress_callback: {}",
            if self.progress_callback.is_some() { "set" } else { "not set" }
        )?;
        Ok(())
    }
}

impl<TTaskProgress> TaskControl<TTaskProgress> {
    #[inline]
    pub fn cancel(&mut self) {
        self.kill_switch.store(true, atomic::Ordering::Relaxed);
    }

    pub fn set_running(&mut self) {
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
