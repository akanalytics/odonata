


#[derive(Copy, Clone, Debug)]
pub enum State {
    NewGame,
    SetPosition,
    StartSearch,
    StartDepthIteration(u32),
}



pub trait Component {
    fn set_state(&mut self, s: State) {
        use State::*;
        match s {
            NewGame => self.new_game(),
            SetPosition => self.new_position(),
            StartSearch => {},
            StartDepthIteration(_) => self.new_iter(),
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

