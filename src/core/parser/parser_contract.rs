use crate::api::http::messages::regex_messages::RegexHttpRequest;
use crate::application::actors::broadcast::RecordBatchWrapper;
use crate::core::error::exception::regex::RegexError;
use serde_json::Value;

pub trait ParserContract: Sync + Send {
    fn parse(
        &self,
        arrow_buffers: Vec<RecordBatchWrapper>,
        regex_request: RegexHttpRequest,
    ) -> Result<Value, RegexError>;
}


pub enum ParserType {
    RUSTREGEX,
    Grok,
    ArrowRegex
}

impl From<String> for ParserType {
    fn from(value: String) -> Self {
        if value.eq("RUSTREGEX") {
            return ParserType::RUSTREGEX;
        }
        if value.eq("ARROWREGEX") {
            return ParserType::ArrowRegex;
        }
        if value.eq("GROK") {
            return ParserType::Grok;
        }
        panic!("Invalid parser type: {}", value);
    }
}