use crate::board::Board;
use crate::catalog::{Catalog, CatalogSuite};
use crate::infra::version::built_info;
use crate::infra::version::Version;
use crate::position::Position;
use crate::search::engine::Engine;
use crate::tags::Tag;
use crate::tuning::Tuning;
use anyhow::Context;
use itertools::Itertools;
// // use crate::{info, logger::LogInit};
// use serde_json::Value;
use jsonrpc_core::{IoHandler, Result};
use jsonrpc_derive::rpc;
use std::fs::File;
use std::io::BufWriter;
use std::sync::Arc;
use std::sync::Mutex;

use super::uci::Uci;

fn to_rpc_error(err: impl Into<anyhow::Error>) -> jsonrpc_core::Error {
    jsonrpc_core::Error {
        message: err.into().to_string(),
        code: jsonrpc_core::ErrorCode::InternalError,
        data: None,
    }
}

#[derive(Debug)]
pub struct JsonRpc {
    io: IoHandler,
}

impl JsonRpc {
    pub fn new(engine: Arc<Mutex<Engine>>) -> JsonRpc {
        let mut me = JsonRpc {
            io: <IoHandler>::new(),
        };
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

    #[rpc(name = "position_catalog")]
    fn position_catalog(&self, suite: CatalogSuite) -> Result<Vec<Position>>;

    #[rpc(name = "position_upload")]
    fn position_upload(&self, filename: String) -> Result<i32>;

    #[rpc(name = "position_download_model")]
    fn position_download_model(&self, filename: String) -> Result<i32>;

    #[rpc(name = "tuning_mean_squared_error")]
    fn tuning_mean_squared_error(&self) -> Result<f32>;

    #[rpc(name = "options")]
    fn options(&self) -> Result<String>;

    #[rpc(name = "eval")]
    fn eval(&self, board: Board) -> Result<Position>;

    #[rpc(name = "static_eval_explain")]
    fn static_eval_explain(&self, board: Board) -> Result<String>;
}

#[derive(Clone, Debug)]
struct RpcImpl {
    // pub tuning: Arc<Mutex<Tuning>>,
    pub engine: Arc<Mutex<Engine>>,
}

impl RpcImpl {
    pub fn new(engine: Arc<Mutex<Engine>>) -> Self {
        RpcImpl {
            engine,
            // tuning: Arc::new(Mutex::new(Tuning::new())),
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

    fn position_catalog(&self, suite: CatalogSuite) -> Result<Vec<Position>> {
        info!("position_catalog({})", suite);
        Ok(Catalog::positions(suite))
    }

    // empty file is clear
    fn position_upload(&self, filename: String) -> Result<i32> {
        let mut eng = self.engine.lock().unwrap();
        if filename.is_empty() {
            eng.tuner.clear();
            info!("Cleared tuner positions");
            return Ok(0);
        }
        info!("Starting tuner upload from {}", &filename);
        let positions = Position::parse_epd_file(&filename).map_err(|s| jsonrpc_core::Error {
            message: format!("{} on uploading positions from '{}'", s, &filename),
            code: jsonrpc_core::ErrorCode::InternalError,
            data: None,
        })?;
        let uploaded_count = Tuning::upload_positions(&mut eng, positions).map_err(to_rpc_error)?;
        info!("Uploaded {} positions", uploaded_count);
        Ok(uploaded_count as i32)
    }

    fn position_download_model(&self, filename: String) -> Result<i32> {
        let f = File::create(&filename)
            .with_context(|| format!("Failed to open file {}", &filename))
            .map_err(to_rpc_error)?;
        let mut f = BufWriter::new(f);
        let mut eng = self.engine.lock().unwrap();
        let line_count = Tuning::write_training_data(&mut eng, &mut f).map_err(to_rpc_error)?;

        Ok(line_count)
    }

    fn tuning_mean_squared_error(&self) -> Result<f32> {
        let eng = self.engine.lock().unwrap();
        let mse = eng
            .tuner
            .calculate_mean_square_error(&eng)
            .map_err(to_rpc_error)?;
        Ok(mse)
    }

    fn options(&self) -> Result<String> {
        let ops = Uci::uci_options(&self.engine.lock().unwrap());
        Ok(ops.iter().join("\n"))
    }

    fn eval(&self, board: Board) -> Result<Position> {
        let res = Tag::Result(board.outcome().as_pgn());
        let mut p = Position::from_board(board);
        p.set(res);
        Ok(p)
    }

    fn static_eval_explain(&self, board: Board) -> Result<String> {
        let explanation = self
            .engine
            .lock()
            .unwrap()
            .algo
            .eval
            .w_eval_explain(&board, false);
        Ok(explanation.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

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

    #[test]
    fn position_download_test() -> anyhow::Result<()> {
        let rpc = RpcImpl::new(Arc::new(Mutex::new(Engine::new())));
        rpc.position_upload("../odonata-extras/epd/quiet-labeled-small.epd".to_string())?;
        let lines = rpc.position_download_model("/tmp/test.csv".to_string())?;
        assert!(lines > 0);
        Ok(())
    }
}

// fn main() {
//   let mut io = IoHandler::new();
//   let rpc = RpcImpl;

//   io.extend_with(rpc.to_delegate());
// }
