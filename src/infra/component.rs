


pub trait Component {
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

