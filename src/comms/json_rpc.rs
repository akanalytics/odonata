use jsonrpc_core::IoHandler;
use serde_json::Value;
use crate::version::Version;



#[derive(Debug, Default)]
pub struct JsonRpc {
    json_handler: IoHandler,
}


impl JsonRpc {

    pub fn new() -> JsonRpc {
        let mut me = Self::default();
        me.json_handler.add_sync_method("version", |_| {
            Ok(Value::String(Version::VERSION.into()))
        });
        me
    }

    pub fn invoke(&mut self, req: &str) -> Option<String> {
        self.json_handler.handle_request_sync(req)
    }
}
