// Fetch actors from the registry

use crate::platform::registry::Registry;
use actix::{Actor, Addr};
use actix::{Handler, Message};
use std::marker::PhantomData;

#[derive(Message)]
#[rtype(result = "Result<Addr<A>, String>")]
pub struct FetchActor<A>
where
    A: Actor<Context = actix::Context<A>> + 'static,
{
    pub _marker: PhantomData<A>,
}

impl<A> FetchActor<A>
where
    A: Actor<Context = actix::Context<A>> + 'static,
{
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<A> Handler<FetchActor<A>> for Registry
where
    A: Actor<Context = actix::Context<A>> + 'static,
    Addr<A>: 'static,
{
    type Result = Result<Addr<A>, String>;

    /// Here, you would implement the logic to return the address for actor type `A`.
    /// This could be from a HashMap, a SystemService, or by starting the actor if it's not running.
    /// For example -
    /// match self.wal_actor_addr.clone() {
    ///     WalActorWrapper::Real(addr) => Ok(addr as Addr<A>),
    ///     _ => {unimplemented!()}
    /// }
    fn handle(&mut self, _msg: FetchActor<A>, _ctx: &mut Self::Context) -> Self::Result {
        unimplemented!()
    }
}
