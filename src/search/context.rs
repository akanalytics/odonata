use std::sync::Arc;
use std::rc::Rc;
use std::cell::RefCell;


#[derive(Debug, Clone ,Default)]
struct Context {
    ids: Option<Ids>,
    qs: Option<QSearch>,
}

#[derive(Clone,Debug, Default)]
struct QSearch {
    q: String,
    context: Rc<RefCell<Context>>,
}

#[derive(Clone,Debug, Default)]
struct Ids {
    ids_ply: RefCell<String>,
    context: Rc<RefCell<Context>>,
}

impl Ids {
    fn call_qsearch(&self) {
        println!("Ids::call_qsearch");
        let a = &self.context.borrow_mut().qs;
        a.as_ref().unwrap().call_ids();
    }
}





impl QSearch {
    fn call_ids(&self) {
        println!("QSearch::call_ids");
        if self.q.is_empty() {
            self.context.borrow_mut().ids.as_ref().unwrap().ids_ply.replace("dog".to_string());
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context() {
        let context = Rc::new(RefCell::new(Context::default()));

        let new_ids = Ids { 
                ids_ply: RefCell::new("cat".to_string()),
                context: Rc::clone(&context),
            };

        let qsearch = QSearch { 
                q: "dog".to_string(),
                context: Rc::clone(&context),
            };

        context.borrow_mut().ids = Some(new_ids);
        context.borrow_mut().qs = Some(qsearch);

        context.borrow_mut().qs.as_ref().unwrap().call_ids();
    }
}



