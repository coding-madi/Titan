use regex::Regex;

use crate::api::http::messages::regex_messages::Pattern as PatternHttp;

#[derive(Debug, Clone)]
pub enum Pattern {
    RegexPattern(RegexPattern),
    GrokPattern(GrokPattern),
}

impl From<PatternHttp> for Pattern {
    fn from(regex_pattern: PatternHttp) -> Self {
        match regex_pattern {
            PatternHttp::RegexPattern(regex_pattern) => Pattern::RegexPattern(RegexPattern {
                override_field: regex_pattern.override_field,
                field: regex_pattern.field,
                pattern_string: regex_pattern.pattern_string.clone(),
                regex: Regex::new(&regex_pattern.pattern_string).unwrap(),
            }),
            PatternHttp::GrokPattern(grok_pattern) => {
                unimplemented!()
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RegexPattern {
    pub override_field: Option<String>,
    pub field: String,
    pub pattern_string: String,
    pub regex: Regex,
}

#[derive(Debug, Clone)]
pub struct GrokPattern {
    pub override_field: Option<String>,
    pub field: String,
    pub pattern_string: String,
    pub regex: Regex,
}
