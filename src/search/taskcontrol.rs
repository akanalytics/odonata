use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone, Default)]
pub struct TaskControl<TCallBackData> {
    progress_callback: Option<Arc<Mutex<dyn FnMut(&TCallBackData) + Send + Sync>>>,
    kill_switch: Arc<AtomicBool>,
    has_been_cancelled: bool,
}

impl<TCallBackData> TaskControl<TCallBackData> {
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

    pub fn invoke_callback(&self, data: &TCallBackData) {
        if let Some(callback) = &self.progress_callback {
            let mut callback = callback.lock().unwrap();
            callback(data);
        }
    }

    pub fn register_callback(&mut self, callback: impl FnMut(&TCallBackData) + Send + Sync + 'static) {
        self.progress_callback = Some(Arc::new(Mutex::new(callback)));
        // let clos = |algo :&Algo| { println!("nps {}", algo.stats().knps()); };
    }
}
