use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use std::sync::Mutex;
use std::fmt;

#[derive(Clone, Default)]
pub struct TaskControl<TTaskProgress> {
    progress_callback: Option<Arc<Mutex<dyn Fn(&TTaskProgress) + Send + Sync>>>,
    kill_switch: Arc<AtomicBool>,
    has_been_cancelled: bool,
}



impl<TTaskProgress> fmt::Display for TaskControl<TTaskProgress> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "kill switch      : {}", self.kill_switch.load(atomic::Ordering::SeqCst))?;
        writeln!(f, "has bn cancelled : {}", self.has_been_cancelled)?;
        writeln!(f, "progress_callback: {}", if self.progress_callback.is_some() { "set" } else { "not set" })?;
        Ok(())
    }
}


impl<TTaskProgress> TaskControl<TTaskProgress> {
    #[inline]
    pub fn cancel(&mut self) {
        self.kill_switch.store(true, atomic::Ordering::SeqCst);
    }

    pub fn set_running(&mut self) {
        self.has_been_cancelled = false;
        self.kill_switch.store(false, atomic::Ordering::SeqCst);
    }

    #[inline]
    pub fn is_cancelled(&mut self) -> bool {
        if !self.has_been_cancelled {
            self.has_been_cancelled = self.kill_switch.load(atomic::Ordering::SeqCst);
        }
        self.has_been_cancelled
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
