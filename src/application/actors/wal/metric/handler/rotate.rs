use crate::application::actors::wal::metric::metric_wal_actor::WalMetricActor;
use actix::{AsyncContext, Handler, Message};
use std::time::Duration;

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct RotateWAL;

impl Handler<RotateWAL> for WalMetricActor {
    type Result = ();

    fn handle(&mut self, _msg: RotateWAL, ctx: &mut Self::Context) -> Self::Result {
        ctx.run_later(Duration::from_secs(0), |actor, _ctx| {
            actor.rotate_file();
        });
    }
}
