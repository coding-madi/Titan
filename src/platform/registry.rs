use actix::{Actor, Addr, Handler, Message};
use tracing::log::trace;

// Core Actor imports
use crate::application::actors::broadcast::{BroadcastActor, BroadcastActorAddr};
use crate::application::actors::db::{DbActor, DbActorAddr};
use crate::application::actors::flight_registry::{FlightRegistry, FlightRegistryActorAddr};
use crate::application::actors::iceberg::{IcebergActor, IcebergActorAddr};
pub(crate) use crate::application::actors::parser::{ParserActorAddr, ParsingActor};
use crate::application::actors::wal::{WalActor, WalActorAddr};

// Test-only mock imports
#[cfg(test)]
use crate::application::actors::broadcast::MockBroadcastActor;
#[cfg(test)]
use crate::application::actors::db::MockDbActor;
#[cfg(test)]
use crate::application::actors::flight_registry::MockFlightRegistry;
#[cfg(test)]
use crate::application::actors::iceberg::MockIcebergActor;
#[cfg(test)]
use crate::application::actors::parser::MockParsingActor;
use crate::application::actors::rhai_meter::{RhaiActor, RhaiActorAddr};
#[cfg(test)]
use crate::application::actors::wal::MockWalActor;

// Registry message definitions
#[derive(Message)]
#[rtype(result = "Result<WalActorAddr, ()>")]
pub struct FetchWalActor;

#[derive(Message)]
#[rtype(result = "Result<ParserActorAddr, ()>")]
pub struct FetchParserActor;

#[derive(Message)]
#[rtype(result = "Result<WalActorAddr, ()>")]
pub struct FlightWalActor;

#[derive(Message)]
#[rtype(result = "Result<BroadcastActorAddr, ()>")]
pub struct FetchBroadcastActor;

#[derive(Message)]
#[rtype(result = "Result<FlightRegistryActorAddr, ()>")]
pub struct FetchFlightRegistryActor;

#[derive(Message)]
#[rtype(result = "Result<IcebergActorAddr, ()>")]
pub struct FetchIcebergActor;

// Registry struct
#[derive(Clone)]
pub struct Registry {
    pub db_actor_addr: DbActorAddr,
    pub broadcast_actor_addr: BroadcastActorAddr,
    pub flight_registry_actor_addr: FlightRegistryActorAddr,
    pub iceberg_actor_addr: IcebergActorAddr,
    pub parser_actor_addr: ParserActorAddr,
    pub wal_actor_addr: WalActorAddr,
}

// Registry Builder
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
        Self {
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
        self.db_actor_addr = Some(DbActorAddr::Real(db_actor.start()));
        self
    }

    #[cfg(test)]
    pub fn db_actor(mut self, db_actor: MockDbActor) -> Self {
        self.db_actor_addr = Some(DbActorAddr::Mock(db_actor.start()));
        self
    }

    pub fn broadcast_actor(mut self, actor: BroadcastActor) -> Self {
        self.broadcast_actor_addr = Some(BroadcastActorAddr::Real(actor.start()));
        self
    }

    #[cfg(test)]
    pub fn broadcast_actor_mock(mut self, actor: MockBroadcastActor) -> Self {
        self.broadcast_actor_addr = Some(BroadcastActorAddr::Mock(actor.start()));
        self
    }

    #[cfg(not(test))]
    pub fn flight_registry_actor(mut self, actor: FlightRegistry) -> Self {
        self.flight_registry_actor_addr = Some(FlightRegistryActorAddr::Real(actor.start()));
        self
    }

    #[cfg(test)]
    pub fn flight_registry_actor(mut self, actor: MockFlightRegistry) -> Self {
        self.flight_registry_actor_addr = Some(FlightRegistryActorAddr::Mock(actor.start()));
        self
    }

    #[cfg(not(test))]
    pub fn iceberg_actor(mut self, actor: IcebergActor) -> Self {
        self.iceberg_actor_addr = Some(IcebergActorAddr::Real(actor.start()));
        self
    }

    #[cfg(test)]
    pub fn iceberg_actor(mut self, actor: MockIcebergActor) -> Self {
        self.iceberg_actor_addr = Some(IcebergActorAddr::Mock(actor.start()));
        self
    }

    #[cfg(not(test))]
    pub fn parser_actor(mut self, actors: Vec<ParsingActor>) -> Self {
        let addrs = actors.into_iter().map(|a| a.start()).collect();
        self.parser_actor_addr = Some(ParserActorAddr::Real(addrs));
        self
    }

    #[cfg(test)]
    pub fn parser_actor(mut self, actors: Vec<MockParsingActor>) -> Self {
        let addrs = actors.into_iter().map(|a| a.start()).collect();
        self.parser_actor_addr = Some(ParserActorAddr::Mock(addrs));
        self
    }

    #[cfg(not(test))]
    pub fn wal_actor(mut self, actor: WalActor) -> Self {
        self.wal_actor_addr = Some(WalActorAddr::Real(actor.start()));
        self
    }

    #[cfg(test)]
    pub fn wal_actor(mut self, actor: MockWalActor) -> Self {
        self.wal_actor_addr = Some(WalActorAddr::Mock(actor.start()));
        self
    }

    pub fn build(self) -> Registry {
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

// Actor impl
impl Actor for Registry {
    type Context = actix::Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        trace!("Registry actor started");
    }
}

// Handlers for setting actors
impl Handler<DbActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: DbActorAddr, _: &mut Self::Context) {
        self.db_actor_addr = msg;
    }
}

impl Handler<BroadcastActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: BroadcastActorAddr, _: &mut Self::Context) {
        self.broadcast_actor_addr = msg;
    }
}

impl Handler<FlightRegistryActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: FlightRegistryActorAddr, _: &mut Self::Context) {
        self.flight_registry_actor_addr = msg;
    }
}

impl Handler<IcebergActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: IcebergActorAddr, _: &mut Self::Context) {
        self.iceberg_actor_addr = msg;
    }
}

impl Handler<ParserActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: ParserActorAddr, _: &mut Self::Context) {
        self.parser_actor_addr = msg;
    }
}

impl Handler<WalActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: WalActorAddr, _: &mut Self::Context) {
        self.wal_actor_addr = msg;
    }
}

// Handlers for fetching actors
impl Handler<FetchWalActor> for Registry {
    type Result = Result<WalActorAddr, ()>;
    fn handle(&mut self, _: FetchWalActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.wal_actor_addr.clone())
    }
}

impl Handler<FetchParserActor> for Registry {
    type Result = Result<ParserActorAddr, ()>;
    fn handle(&mut self, _: FetchParserActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.parser_actor_addr.clone())
    }
}

impl Handler<FetchBroadcastActor> for Registry {
    type Result = Result<BroadcastActorAddr, ()>;
    fn handle(&mut self, _: FetchBroadcastActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.broadcast_actor_addr.clone())
    }
}

impl Handler<FetchFlightRegistryActor> for Registry {
    type Result = Result<FlightRegistryActorAddr, ()>;
    fn handle(&mut self, _: FetchFlightRegistryActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.flight_registry_actor_addr.clone())
    }
}

impl Handler<FetchIcebergActor> for Registry {
    type Result = Result<IcebergActorAddr, ()>;
    fn handle(&mut self, _: FetchIcebergActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.iceberg_actor_addr.clone())
    }
}
