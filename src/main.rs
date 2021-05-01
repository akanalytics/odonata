use odonata::comms::console::Console;
use env_logger;


fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    Console::run();
}
    
    
