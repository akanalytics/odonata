


let task = Task<Algo>
task.set_callback();
task.run()



// type AlgoSender = mpsc::Sender<String>;

type Func = dyn FnMut(&Algo) + Send + Sync;
type Callback = Arc<Mutex<Func>>;





pub struct Task<T> {
    task: T,
    callback: Option<Callback>,
    child_thread: MyThreadHandle,
    kill: Arc<atomic::AtomicBool>,
}






impl Task<T> {

    //pub fn add_callback(&mut self, callback: dyn FnMut(String) -> bool + Send + Sync) -> Self {
    //}

    pub fn set_callback(&mut self, callback: Callback) -> Self {
        self.callback = Some(callback);
        self.clone()
     }


     pub fn async_run(&mut self)  {

        const FOUR_MB: usize = 4 * 1024 * 1024;
        let name = String::from("task-thread");
        let builder = thread::Builder::new().name(name).stack_size(FOUR_MB);
        let mut t = self.clone();
        self.child_thread = MyThreadHandle(Some(builder.spawn(move || algo.search(board)).unwrap()));
     }
  
  
       fn invoke_callback(&self) {
        if let Some(func) = &self.callback {
            let mut func = func.lock().unwrap();
            func(self);
        }
    }


    pub fn cancel(&mut self) {
        self.cancel();
        let mut option_thread = self.child_thread.0.take();
        let handle = option_thread.take().unwrap();

        // wait for thread to cancel 
        let algo = handle.join().unwrap();
        self.stats = algo.stats;
        self.pv = algo.pv;
        self.score = algo.score;
        self.clock = algo.clock;
    }
    }




#[derive(Debug, Default)]
struct MyThreadHandle(Option<thread::JoinHandle<Algo>>);

impl Clone for MyThreadHandle {
    fn clone(&self) -> Self {
        Self(None)
    }
}
