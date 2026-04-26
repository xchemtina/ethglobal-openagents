//! Foreign-language subprocess skill worker placeholder.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerCommand {
    pub executable: String,
    pub args: Vec<String>,
}

pub trait WorkerCodec {
    fn encode_request(&self, payload: &[u8]) -> Vec<u8>;
    fn decode_response(&self, payload: &[u8]) -> Result<Vec<u8>, String>;
}
