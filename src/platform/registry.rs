#[cfg(test)]
use crate::application::actors::broadcast::MockBroadcastActor;
use crate::application::actors::broadcast::{BroadcastActor, BroadcastActorAddr};
#[cfg(test)]
use crate::application::actors::db::DbActorAddr::Mock;
#[cfg(not(test))]
use crate::application::actors::db::DbActorAddr::Real;
#[cfg(test)]
use crate::application::actors::db::MockDbActor;
use crate::application::actors::db::{DbActor, DbActorAddr};
#[cfg(test)]
use crate::application::actors::flight_registry::MockFlightRegistry;
use crate::application::actors::flight_registry::{FlightRegistry, FlightRegistryActorAddr};
#[cfg(test)]
use crate::application::actors::iceberg::MockIcebergActor;
use crate::application::actors::iceberg::{IcebergActor, IcebergActorAddr};
#[cfg(test)]
use crate::application::actors::parser::MockParsingActor;
pub(crate) use crate::application::actors::parser::ParserActorAddr;
use crate::application::actors::parser::ParsingActor;
#[cfg(test)]
use crate::application::actors::wal::MockWalActor;
use crate::application::actors::wal::{WalActor, WalActorAddr};
use actix::{Actor, Addr, Handler, Message};
use tracing::log::trace;

#[derive(Clone)]
pub struct Registry {
    pub db_actor_addr: DbActorAddr,
    pub broadcast_actor_addr: BroadcastActorAddr,
    pub flight_registry_actor_addr: FlightRegistryActorAddr,
    pub iceberg_actor_addr: IcebergActorAddr,
    pub parser_actor_addr: ParserActorAddr,
    pub wal_actor_addr: WalActorAddr,
}

pub struct RegistryBuilder {
    db_actor_addr: Option<DbActorAddr>,
    broadcast_actor_addr: Option<BroadcastActorAddr>,
    flight_registry_actor_addr: Option<FlightRegistryActorAddr>,
    iceberg_actor_addr: Option<IcebergActorAddr>,
    parser_actor_addr: Option<ParserActorAddr>,
    wal_actor_addr: Option<WalActorAddr>,
}

impl RegistryBuilder {
    pub fn new() -> Self {
        RegistryBuilder {
            db_actor_addr: None,
            broadcast_actor_addr: None,
            flight_registry_actor_addr: None,
            iceberg_actor_addr: None,
            parser_actor_addr: None,
            wal_actor_addr: None,
        }
    }

    #[cfg(not(test))]
    pub fn db_actor(mut self, db_actor: DbActor) -> Self {
        self.db_actor_addr = Some(Real(db_actor.start()));
        self
    }

    #[cfg(test)]
    pub fn db_actor(mut self, db_actor: MockDbActor) -> Self {
        self.db_actor_addr = Some(Mock(db_actor.start()));
        self
    }

    // #[cfg(not(test))]
    pub fn broadcast_actor(mut self, broadcast_actor: BroadcastActor) -> Self {
        self.broadcast_actor_addr = Some(BroadcastActorAddr::Real(broadcast_actor.start()));
        self
    }

    #[cfg(test)]
    pub fn broadcast_actor_mock(mut self, broadcast_actor: MockBroadcastActor) -> Self {
        self.broadcast_actor_addr = Some(BroadcastActorAddr::Mock(broadcast_actor.start()));
        self
    }

    #[cfg(not(test))]
    pub fn flight_registry_actor(mut self, flight_registry_actor: FlightRegistry) -> Self {
        self.flight_registry_actor_addr =
            Some(FlightRegistryActorAddr::Real(flight_registry_actor.start()));
        self
    }

    #[cfg(test)]
    pub fn flight_registry_actor(mut self, flight_registry_actor: MockFlightRegistry) -> Self {
        self.flight_registry_actor_addr =
            Some(FlightRegistryActorAddr::Mock(flight_registry_actor.start()));
        self
    }

    #[cfg(not(test))]
    pub fn iceberg_actor(mut self, iceberg_actor: IcebergActor) -> Self {
        self.iceberg_actor_addr = Some(IcebergActorAddr::Real(iceberg_actor.start()));
        self
    }

    #[cfg(test)]
    pub fn iceberg_actor(mut self, iceberg_actor: MockIcebergActor) -> Self {
        self.iceberg_actor_addr = Some(IcebergActorAddr::Mock(iceberg_actor.start()));
        self
    }

    #[cfg(not(test))]
    pub fn parser_actor(mut self, parser_actor: Vec<ParsingActor>) -> Self {
        let mut parser_actors_addr: Vec<Addr<ParsingActor>> = vec![];
        for actor in parser_actor {
            parser_actors_addr.push(actor.start());
        }
        self.parser_actor_addr = Some(ParserActorAddr::Real(parser_actors_addr));
        self
    }

    #[cfg(test)]
    pub fn parser_actor(mut self, parser_actor: Vec<MockParsingActor>) -> Self {
        let mut parser_actors_addr: Vec<Addr<MockParsingActor>> = vec![];
        for actor in parser_actor {
            parser_actors_addr.push(actor.start());
        }
        self.parser_actor_addr = Some(ParserActorAddr::Mock(parser_actors_addr));
        self
    }

    #[cfg(not(test))]
    pub fn wal_actor(mut self, wal_actor: WalActor) -> Self {
        let wal_actor_addr = wal_actor.start();
        self.wal_actor_addr = Some(WalActorAddr::Real(wal_actor_addr));
        self
    }

    #[cfg(test)]
    pub fn wal_actor(mut self, wal_actor: MockWalActor) -> Self {
        let wal_actor_addr = wal_actor.start();
        self.wal_actor_addr = Some(WalActorAddr::Mock(wal_actor_addr));
        self
    }

    pub fn build(self) -> Registry {
        // Here, you'd typically handle cases where required fields aren't set.
        // For simplicity, we'll unwrap, but in production, you might return a Result
        // or provide default values.
        Registry {
            db_actor_addr: self.db_actor_addr.expect("db_actor_addr must be set"),
            broadcast_actor_addr: self
                .broadcast_actor_addr
                .expect("broadcast_actor_addr must be set"),
            flight_registry_actor_addr: self
                .flight_registry_actor_addr
                .expect("flight_registry_actor_addr must be set"),
            iceberg_actor_addr: self
                .iceberg_actor_addr
                .expect("iceberg_actor_addr must be set"),
            parser_actor_addr: self
                .parser_actor_addr
                .expect("parser_actor_addr must be set"),
            wal_actor_addr: self.wal_actor_addr.expect("wal_actor_addr must be set"),
        }
    }
}

impl Actor for Registry {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        trace!("Registry actor started");
    }
}

impl Handler<DbActorAddr> for Registry {
    type Result = ();

    fn handle(&mut self, msg: DbActorAddr, ctx: &mut Self::Context) -> Self::Result {
        self.db_actor_addr = msg;
    }
}

impl Handler<ParserActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: ParserActorAddr, ctx: &mut Self::Context) -> Self::Result {
        self.parser_actor_addr = msg;
    }
}

impl Handler<BroadcastActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: BroadcastActorAddr, ctx: &mut Self::Context) -> Self::Result {
        self.broadcast_actor_addr = msg;
    }
}

impl Handler<FlightRegistryActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: FlightRegistryActorAddr, ctx: &mut Self::Context) -> Self::Result {
        self.flight_registry_actor_addr = msg;
    }
}

impl Handler<IcebergActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: IcebergActorAddr, ctx: &mut Self::Context) -> Self::Result {
        self.iceberg_actor_addr = msg;
    }
}

impl Handler<WalActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: WalActorAddr, ctx: &mut Self::Context) -> Self::Result {
        self.wal_actor_addr = msg;
    }
}

impl Handler<FetchParserActor> for Registry {
    type Result = Result<ParserActorAddr, ()>;

    fn handle(&mut self, _msg: FetchParserActor, ctx: &mut Self::Context) -> Self::Result {
        Ok(self.parser_actor_addr.clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<WalActorAddr, ()>")]
pub struct FetchWalActor;

impl Handler<FetchWalActor> for Registry {
    type Result = Result<WalActorAddr, ()>;

    fn handle(&mut self, msg: FetchWalActor, ctx: &mut Self::Context) -> Self::Result {
        Ok(self.wal_actor_addr.clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<ParserActorAddr, ()>")]
pub struct FetchParserActor;

#[derive(Message)]
#[rtype(result = "Result<WalActorAddr, ()>")]
pub struct FlightWalActor;


#[derive(Message)]
#[rtype(result = "Result<BroadcastActorAddr, ()>")]
pub struct FetchBroadcastActor;

impl Handler<FetchBroadcastActor> for Registry {
    type Result = Result<BroadcastActorAddr, ()>;

    fn handle(&mut self, msg: FetchBroadcastActor, ctx: &mut Self::Context) -> Self::Result {
        Ok(self.broadcast_actor_addr.clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<FlightRegistryActorAddr, ()>")]
pub struct FetchFlightRegistryActor;

impl Handler<FetchFlightRegistryActor> for Registry {
    type Result = Result<FlightRegistryActorAddr, ()>;

    fn handle(&mut self, msg: FetchFlightRegistryActor, ctx: &mut Self::Context) -> Self::Result {
        Ok(self.flight_registry_actor_addr.clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<IcebergActorAddr, ()>")]
pub struct FetchIcebergActor;

impl Handler<FetchIcebergActor> for Registry {
    type Result = Result<IcebergActorAddr, ()>;

    fn handle(&mut self, msg: FetchIcebergActor, ctx: &mut Self::Context) -> Self::Result {
        Ok(self.iceberg_actor_addr.clone())
    }
}