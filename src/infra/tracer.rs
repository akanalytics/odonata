#[derive(Clone, Copy)]
pub struct NullTracer;

#[derive(Clone, Copy)]
pub struct LoggingTracer;

#[derive(Clone, Copy)]
pub struct FileTracer;

pub trait Tracer {
    fn trace<D: ?Sized + Traceable>(&self, d: &D) -> &Self;
}

pub trait Traceable {
    fn log(&self, t: &LoggingTracer);
    fn file(&self, t: &FileTracer);
}

impl Tracer for LoggingTracer {
    fn trace<D: ?Sized + Traceable>(&self, d: &D) -> &LoggingTracer {
        d.log(self);
        self
    }
}

impl Tracer for FileTracer {
    fn trace<D: ?Sized + Traceable>(&self, d: &D) -> &FileTracer {
        d.file(self);
        self
    }
}

impl Tracer for NullTracer {
    fn trace<D: ?Sized + Traceable>(&self, _d: &D) -> &NullTracer {
        self
    }
}

impl Traceable for str {
    fn log(&self, _t: &LoggingTracer) {
        println!("string '{}'", self);
    }
    fn file(&self, _t: &FileTracer) {
        println!("string '{}'", self);
    }
}

impl Traceable for i32 {
    fn log(&self, _t: &LoggingTracer) {
        println!("int {}", self);
    }
    fn file(&self, _t: &FileTracer) {
        println!("int '{}'", self);
    }
}

// impl<D: ?Sized> Trace<D> for NullTracer {
//     fn trace(&self, _s: &D) -> &NullTracer {
//         self
//     }
// }

// impl Trace<i32> for LoggingTracer {
//     fn trace(&self, i: &i32) -> &LoggingTracer {
//         print!("{}_i32 ", i);
//         self
//     }
// }

// impl Trace<str> for LoggingTracer {
//     fn trace(&self, s: &str) -> &LoggingTracer {
//         print!("{}", s);
//         self
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace() {
        let nt = NullTracer;
        nt.trace("Hello").trace(&32).trace(&45);

        let lt = LoggingTracer;
        lt.trace("45=").trace(&45).trace("and 32=").trace(&32).trace("\n");
    }
}
