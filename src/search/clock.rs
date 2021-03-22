use std::sync::atomic;
use std::sync::Arc;
use std::time;


#[derive(Debug, Clone, Default)]
pub struct Clock {
    start_time: Option<time::Instant>,
    kill: Arc<atomic::AtomicBool>,
}


impl Clock {
    pub fn start(&mut self) {
        self.start_time = Some(time::Instant::now());
    }

    /// will panic if clock not started
    pub fn elapsed(&self) -> time::Duration {
        self.start_time.unwrap().elapsed()
    }

    pub fn time_up(&self) -> bool {
        // FIXME: time logic too
        self.kill.load(atomic::Ordering::Relaxed)
    }



}



