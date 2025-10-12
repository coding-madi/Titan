use crate::application::actors::broadcaster::broadcast_actor::RecordBatchWrapper;
use crate::core::error::exception::regex::RegexError;
use crate::core::parser::messages::parser::Pattern;
use std::collections::HashMap;

pub trait ParserContract: Sync + Send {
    fn parse(
        &self,
        arrow_buffers: Vec<RecordBatchWrapper>,
        pattern: HashMap<String, Vec<Pattern>>,
        sample: bool,
    ) -> Result<Vec<RecordBatchWrapper>, RegexError>;
}

pub enum ParserType {
    RUSTREGEX,
    Grok,
    ArrowRegex,
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
