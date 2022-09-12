use std::sync::atomic::AtomicBool;

pub static FEATURE: AtomicBool = AtomicBool::new(false);

#[derive(Copy, Clone, Debug)]
pub enum State {
    NewGame,
    SetPosition,
    StartSearch,
    EndSearch,
    StartDepthIteration(i32),
    Shutdown,
}

pub trait Component {
    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame => self.new_game(),
            SetPosition => self.new_position(),
            StartSearch => {}
            EndSearch => {}
            StartDepthIteration(_) => self.new_iter(),
            Shutdown => {}
        }
    }
    fn new_game(&mut self);
    fn new_iter(&mut self) {}
    fn new_position(&mut self);
    fn set_thread_index(&mut self, _thread_index: u32) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestStruct {
        integer: i64,
        string: String,
    }

    impl Component for TestStruct {
        fn new_game(&mut self) {}
        fn new_iter(&mut self) {}
        fn new_position(&mut self) {}
    }
}
