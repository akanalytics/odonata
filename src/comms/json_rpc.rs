use crate::version::Version;
use crate::catalog::{Catalog, CatalogSuite};
use crate::position::Position;
use crate::board::Board;
use crate::tags::Tag;
use crate::{logger::LogInit,info};
// use serde_json::Value;
use jsonrpc_core::{IoHandler, Result};
use jsonrpc_derive::rpc;




#[derive(Debug, Default)]
pub struct JsonRpc {
    io: IoHandler,
}


impl JsonRpc {

    pub fn new() -> JsonRpc {
        let mut me = JsonRpc::default();   // { io: <IoHandler as Trait>::new() };
        let rpc = RpcImpl;
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




#[rpc(server)]
pub trait Rpc {
	#[rpc(name = "version")]
	fn version(&self) -> Result<String>;

	#[rpc(name = "positionsCatalog")]
	fn positions_catalog(&self, suite: CatalogSuite) -> Result<Vec<Position>>;

	#[rpc(name = "eval")]
	fn eval(&self, board: Board) -> Result<Position>;
}


struct RpcImpl;
impl Rpc for RpcImpl {
    fn version(&self) -> Result<String> {
        Ok(Version::VERSION.into())
    }

	fn positions_catalog(&self, suite: CatalogSuite) -> Result<Vec<Position>> {
        info!("positions_catalog({})", suite);
        Ok(Catalog::positions(suite))
    }

	fn eval(&self, board: Board) -> Result<Position> {
        let res = Tag::Result( board.outcome().as_pgn());
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
        let mut rpc = JsonRpc::new();
        let request1 = r#"{"jsonrpc": "2.0", "method": "version", "params": [], "id": 1}"#;
        let response = String::from(r#"{"jsonrpc":"2.0","result":""#) + Version::VERSION + r#"","id":1}"#;
        assert_eq!(rpc.invoke(request1), Some(response.to_string()));        
    }
}


// fn main() {
//   let mut io = IoHandler::new();
//   let rpc = RpcImpl;

//   io.extend_with(rpc.to_delegate());
// }