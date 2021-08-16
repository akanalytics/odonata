use std::sync::Arc;
use std::sync::Mutex;

use crate::board::Board;
use crate::catalog::{Catalog, CatalogSuite};
use crate::position::Position;
use crate::search::algo::Engine;
use crate::tags::Tag;
use crate::tuning::Tuning;
use crate::version::built_info;
use crate::version::Version;
use crate::{info, logger::LogInit};
// use serde_json::Value;
use jsonrpc_core::{IoHandler, Result};
use jsonrpc_derive::rpc;

#[derive(Debug)]
pub struct JsonRpc {
    io: IoHandler,
}



impl JsonRpc {
    pub fn new(engine: Arc<Mutex<Engine>>) -> JsonRpc {
        let mut me = JsonRpc { io: <IoHandler>::new() };
        let rpc = RpcImpl::new(engine);
        me.io.extend_with(rpc.to_delegate());

        // me.io.add_sync_method("version", |_| {
        //     Ok(Value::String(Version::VERSION.into()))
        // });

        // me.io.add_sync_method("positionsCatalog", |name| {
        //     Ok(Value::String(Version::VERSION.into()))
        // });

        me
    }

    pub fn invoke(&mut self, req: &str) -> Option<String> {
        self.io.handle_request_sync(req)
    }
}


// members must be threadsafe
// interior mutability since non-mut self
#[rpc(server)]
pub trait Rpc {
    #[rpc(name = "version")]
    fn version(&self) -> Result<String>;

    #[rpc(name = "positionsCatalog")]
    fn positions_catalog(&self, suite: CatalogSuite) -> Result<Vec<Position>>;

    #[rpc(name = "position_upload")]
    fn position_upload(&self, filename: String) -> Result<()>;

    #[rpc(name = "tuning_mean_squared_error")]
    fn tuning_mean_squared_error(&self) -> Result<f32>;

    #[rpc(name = "eval")]
    fn eval(&self, board: Board) -> Result<Position>;
}

#[derive(Clone, Debug)]
struct RpcImpl {
    pub tuning: Arc<Mutex<Tuning>>,
    pub engine: Arc<Mutex<Engine>>,
}

impl RpcImpl {
    pub fn new(engine: Arc<Mutex<Engine>>) -> Self {
        RpcImpl {
            engine,
            tuning: Arc::new(Mutex::new(Tuning::new())),
        }
    }
}

impl Rpc for RpcImpl {
    fn version(&self) -> Result<String> {
        Ok(format!(
            "{} {} built on {}",
            Version::NAME,
            Version::VERSION,
            built_info::BUILT_TIME_UTC
        ))
    }

    fn positions_catalog(&self, suite: CatalogSuite) -> Result<Vec<Position>> {
        info!("positions_catalog({})", suite);
        Ok(Catalog::positions(suite))
    }

    fn position_upload(&self, filename: String) -> Result<()> {
        self.tuning.lock().unwrap().positions =
            Position::parse_epd_file(filename).map_err(|s| jsonrpc_core::Error {
                message: s,
                code: jsonrpc_core::ErrorCode::InternalError,
                data: None,
            })?;
        Ok(())
    }

    fn tuning_mean_squared_error(&self) -> Result<f32> {
        let mse = self
            .tuning
            .lock()
            .unwrap()
            .calculate_mean_square_error(&self.engine.lock().unwrap());
        Ok(mse)
    }

    fn eval(&self, board: Board) -> Result<Position> {
        let res = Tag::Result(board.outcome().as_pgn());
        let mut p = Position::from_board(board);
        p.set(res);
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc() {
        let mut rpc = JsonRpc::new(Arc::new(Mutex::new(Engine::new())));
        let request1 = r#"{"jsonrpc": "2.0", "method": "version", "params": [], "id": 1}"#;
        let response = format!(
            r#"{{"jsonrpc":"2.0","result":"{} {} built on {}","id":1}}"#,
            Version::NAME,
            Version::VERSION,
            built_info::BUILT_TIME_UTC
        );
        assert_eq!(rpc.invoke(request1), Some(response.to_string()));
    }
}

// fn main() {
//   let mut io = IoHandler::new();
//   let rpc = RpcImpl;

//   io.extend_with(rpc.to_delegate());
// }
