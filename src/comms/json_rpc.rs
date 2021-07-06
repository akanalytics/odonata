use crate::version::Version;
use crate::catalog::Catalog;
use crate::position::Position;
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
	fn positions_catalog(&self, name: String) -> Result<Vec<Position>>;

}


struct RpcImpl;
impl Rpc for RpcImpl {
    fn version(&self) -> Result<String> {
        Ok(Version::VERSION.into())
    }

	fn positions_catalog(&self, _name: String) -> Result<Vec<Position>> {
        let positions = Catalog::win_at_chess();
        Ok(positions)
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc() {
        let rpc = JsonRpc::new();
        let request1 = r#"{"jsonrpc": "2.0", "method": "version", "params": [], "id": 1}"#;
        let response = "?";
        assert_eq!(rpc.invoke(request1), Some(response.to_string()));        
    }
}


// fn main() {
//   let mut io = IoHandler::new();
//   let rpc = RpcImpl;

//   io.extend_with(rpc.to_delegate());
// }