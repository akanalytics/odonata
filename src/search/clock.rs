use std::sync::atomic;
use std::sync::Arc;
use std::time;
use std::fmt;


#[derive(Debug, Clone, Default)]
pub struct Clock {
    start_time: Option<time::Instant>,
    kill: Arc<atomic::AtomicBool>,
    clock_checks: u64,
    was_killed: bool,
}


impl Clock {
    pub fn start(&mut self) {
        self.start_time = Some(time::Instant::now());
    }

    /// will panic if clock not started
    pub fn elapsed(&self) -> time::Duration {
        self.start_time.unwrap().elapsed()
    }

    pub fn time_up(&mut self) -> bool {
        // FIXME: using SeqCst until i think this through more
        self.clock_checks += 1;
        let time_up = self.kill.load(atomic::Ordering::SeqCst);
        if time_up {
            self.was_killed = true;
        }
        time_up
    }

    pub fn set_time_up(&mut self) {
        // FIXME: using SeqCst until i think this through more
        self.kill.store(true, atomic::Ordering::SeqCst);
    }


}

impl fmt::Display for Clock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "start time       :{:?}", self.start_time)?;
        writeln!(f, "clock_checks     :{}", self.clock_checks)?;
        writeln!(f, "kill             :{}", self.kill.load(atomic::Ordering::SeqCst))?;
        writeln!(f, "was_killed       :{}", self.was_killed)?;
        writeln!(f, "count            :{}", Arc::strong_count(&self.kill))?;
        Ok(())
    }
}

