





trait Tracer {
    fn enabled() -> bool;
    fn trace(x: i32, y: i32, s: &str);
}


struct NullTracer;

impl Tracer for NullTracer {
    #[inline]
    fn enabled() -> bool {
        false
    }
    fn trace(_x: i32, _y: i32, _s: &str) {
    }
}

struct LoggingTracer;
impl Tracer for LoggingTracer {
    #[inline]
    fn enabled() -> bool {
        true
    }
    fn trace(x: i32, y: i32, s: &str) {
        println!("x={} y={} s={}", x, y, s);
    }
}



// #[derive(Clone,Debug)]
// pub struct Debug {
//     board: Board,
//     items: Vec<String>,
// }
