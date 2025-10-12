use crate::application::actors::broadcaster::broadcast_actor::{
    BroadcastActor, submit_regex_request,
};
use crate::application::actors::parser::handlers::try_parsing_regex::TryParsingRegex;
use crate::core::error::exception::regex::RegexError;
use actix::{ActorFutureExt, Handler, ResponseActFuture, WrapFuture};
use serde_json::Value;

/// The difference SubmitRegexRequest and TryParsingRegex is that,
/// SubmitRegexRequest simply submits the request and does not validate the regex against real data.
/// TryParsingRegex fetches the data from cache and applies the regex over the data
impl Handler<TryParsingRegex> for BroadcastActor {
    type Result = ResponseActFuture<Self, Result<Value, RegexError>>;

    fn handle(&mut self, msg: TryParsingRegex, _ctx: &mut Self::Context) -> Self::Result {
        let parsers = self.parsers.clone();
        let async_task = async move {
            let values = submit_regex_request(parsers, msg).await;
            let final_json = serde_json::json!({
                "handler": "Processed results from multiple parsers",
                "results": values,
            });

            Ok(final_json)
        };

        async_task.into_actor(self).boxed_local()
    }
}
