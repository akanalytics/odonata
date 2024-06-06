use std::cell::Cell;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Context, Result};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Param {
    pub field:           String,
    pub key:             String,
    pub value:           String,
    pub total_mod_count: Arc<Cell<u32>>,
    pub children:        Arc<Cell<u32>>,
    pub parent:          Arc<Cell<u32>>,
}

pub trait Configurable {
    fn set(&mut self, p: Param) -> Result<bool>;
}

impl<T> Configurable for T
where
    T: FromStr,
    anyhow::Error: From<T::Err>,
{
    fn set(&mut self, p: Param) -> Result<bool> {
        if p.field == p.key {
            *self = T::from_str(&p.value)
                .map_err(anyhow::Error::from)
                .context(format!("applying {p} to field"))?;
            p.total_mod_count.set(p.total_mod_count.get() + 1);
            p.parent.set(p.parent.get() + 1);
            info!(target: "config", "applied {p} to field");
            Ok(true)
        } else {
            debug!(target: "config", "tried {p} on field {}", p.field);
            Ok(false)
        }
        // let e = crate::domain::score::Score::parse_pgn_pawn_value("").context("")
    }
}

impl Display for Param {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}={}", self.key, self.value)
    }
}

impl Param {
    pub fn new(key: &str, value: &str) -> Self {
        Self {
            key: key.to_string(),
            value: value.to_string(),
            ..Self::default()
        }
    }

    pub fn is_modified(&self) -> bool {
        if self.total_mod_count.get() - self.children.get() > 0 {
            info!("{} was modified", self.key);
            true
        } else {
            false
        }
    }

    pub fn get(&self, field: &str) -> Self {
        let mut me = self.clone();
        me.children = Arc::default();
        me.parent = self.children.clone();
        if !me.field.is_empty() {
            me.field.push('.');
        }
        me.field += field;
        me
    }
}

#[cfg(test)]
mod test {
    use test_log::test;

    use super::*;

    #[test]
    fn test_config_param() {
        let p = Param::new("big.small.field", "1");

        let q = p.clone();
        p.get("nested").total_mod_count.set(1);
        assert_eq!(q.total_mod_count.get(), 1);

        assert_eq!(p.get("field").field, "field");
        assert_eq!(p.get("field").key, "big.small.field");
        assert_eq!(p.get("big").get("small").get("field").field, "big.small.field");
        assert_eq!(p.get("big").get("small").get("field").key, "big.small.field");
        assert_eq!(p, q);
        1.set(p.get("big.small.field")).unwrap();
        println!("{p:?}");
        assert_eq!(p.total_mod_count.get(), 2);

        let p0 = Param::new("o1.o2.f1", "5");
        1.set(p0.get("f1")).unwrap();
        let p1 = p0.get("o1");
        let p2 = p1.get("o2");
        1.set(p2.get("f1")).unwrap();
        assert_eq!(p2.children.get(), 1);
        assert_eq!(p1.children.get(), 0);
        assert_eq!(p0.children.get(), 0);

        println!("{p:?} {:?}", p.get("max_depth"));
    }
}

// pub fn set_field<T>(&self, field: &str, t: &mut T) -> Result<bool>
// where
//     T: FromStr,
//     // T::Err: Sync + Send + 'static,
//     anyhow::Error: From<T::Err>,
// {
//     if field == self.key {
//         *t = T::from_str(&self.value)
//             .map_err(anyhow::Error::from)
//             .context(format!("applying {self} to field {field}"))?;
//         *self.applied.lock().unwrap() += 1;
//         info!(target: "eng", "applied {self} to field {field} ");
//         Ok(true)
//     } else {
//         Ok(false)
//     }
//     // let e = crate::domain::score::Score::parse_pgn_pawn_value("").context("")
// }
// impl ConfigParam {
//     pub fn set_field<T>(&self, field: &str, t: &mut T) -> Result<bool>
//     where
//         T: FromStr,
//         T::Err: Sync + Send + 'static,
//         anyhow::Error: From<T::Err>,
//     {
//         if field == self.key {
//             let res: Result<_, T::Err> = <T as FromStr>::from_str(&self.value);
//             match res {
//                 Ok(n) => {
//                     *t = n;
//                     *self.applied.lock().unwrap() += 1;
//                     info!(target: "eng", "applied {self} to field {field} ");
//                     Ok(true)
//                 }
//                 Err(e) => Err(anyhow::Error::from(e).context(format!("applying {self} to field {field}"))),
//             }
//         } else {
//             Ok(false)
//         }
//         // let e = crate::domain::score::Score::parse_pgn_pawn_value("").context("")
//     }

// pub fn set_struct<T>(&self, field: &str, t: &mut T) -> Result<bool>
// where
//     T: Configurable,
//     // T::Err: Sync + Send + std::error::Error + 'static,
// {
//     let param = Self {
//         key:     self.key.trim_start_matches(field).to_string(),
//         value:   self.value.clone(),
//         applied: Default::default(),
//     };
//     t.set_all_fields(self)?;
//     let applied = *param.applied.lock().unwrap();
//     *self.applied.lock().unwrap() += applied;
//     Ok(applied > 0)
// }
