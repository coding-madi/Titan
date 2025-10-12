use crate::application::actors::broadcaster::broadcast_actor::{
    BroadcastActor, submit_regex_request,
};
use crate::application::actors::parser::parser_actor::SubmitRegexRequest;
use crate::core::error::exception::regex::RegexError;
use actix::{ActorFutureExt, Handler, ResponseActFuture, WrapFuture};
use serde_json::Value;

impl Handler<SubmitRegexRequest> for BroadcastActor {
    type Result = ResponseActFuture<Self, Result<Value, RegexError>>;
    fn handle(
        &mut self,
        regex_request: SubmitRegexRequest,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        // Correct: Clone the parsers to move them into the async block.
        let parsers = self.parsers.clone();
        let async_task = async move {
            let values = submit_regex_request(parsers, regex_request).await;
            let final_json = serde_json::json!({
                "handler": "Processed results from multiple parsers",
                "results": values,
            });
            Ok(final_json)
        };

        // Correct: Use `into_actor(self)`. The `actix` framework handles the
        // ownership of the actor for the duration of the future.
        async_task.into_actor(self).boxed_local()
    }
}
