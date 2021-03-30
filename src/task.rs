use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::sync::atomic;
use crate::search::algo::Algo;



// let task = Task<Algo>
// task.set_callback();
// task.run()



// type AlgoSender = mpsc::Sender<String>;

// type TCallback = impl FnMut(&Algo) + Send + Sync;
// type Callback = Arc<Mutex<Func>>;




#[derive(Clone, Default)]
pub struct CancellableTask<TCallbackData> {
    callback: Option<Arc<Mutex<dyn FnMut(&TCallbackData) + Send + Sync>>>,
    kill: Arc<atomic::AtomicBool>,
    is_cancelled: bool, 
}


impl<TCallbackData: Clone> CancellableTask<TCallbackData> {

    //pub fn add_callback(&mut self, callback: dyn FnMut(String) -> bool + Send + Sync) -> Self {
    //}

    pub fn set_callback(&mut self, callback: impl FnMut(&TCallbackData) + Send + Sync + 'static) -> Self {
        self.callback = Some(Arc::new(Mutex::new(callback)));
        self.clone()
     }


    //  pub fn async_run(&mut self)  {

    //     const FOUR_MB: usize = 4 * 1024 * 1024;
    //     let name = String::from("task-thread");
    //     let builder = thread::Builder::new().name(name).stack_size(FOUR_MB);
    //     let mut t = self.clone();
    //     self.child_thread = MyThreadHandle(Some(builder.spawn(move || algo.search(board)).unwrap()));
    //  }
  
  
    fn invoke_callback(&self, t: &TCallbackData) {
        if let Some(func) = &self.callback {
            let mut func = func.lock().unwrap();
            func(t);
        }
    }

    // pub fn cancel(&mut self) {
    //     if !self.is_cancelled() {
    //         self.kill.store(true, atomic::Ordering::SeqCst);
    //     }
    // }
}



// #[inline]
// fn cancelled(&mut self) -> bool {
//     let time_up = self.kill.load(atomic::Ordering::SeqCst);
//     time_up
// }



// pub fn cancel(&mut self) {
//     self.cancel();
//     let mut option_thread = self.child_thread.0.take();
//     let handle = option_thread.take().unwrap();

//     // wait for thread to cancel 
//     let algo = handle.join().unwrap();


// child_thread: MyThreadHandle,

#[derive(Debug, Default)]
struct MyThreadHandle(Option<thread::JoinHandle<Algo>>);

impl Clone for MyThreadHandle {
    fn clone(&self) -> Self {
        Self(None)
    }
}


// Search =  builder

// TaskRunner.set_callback(||)
// TaskRunner.run(search);
// TaskRunner.cancel()
// TaskRunner.results()



