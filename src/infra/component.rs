


pub trait Component {
    fn new_game(&mut self);
    fn new_iter(&mut self) {}
    fn new_position(&mut self);
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

