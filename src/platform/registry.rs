use actix::{Actor, Handler, Message};
use std::collections::HashMap;
use std::io::Error;
use tracing::log::trace;
use tracing::{error, info};

// Core Actor imports
use crate::application::actors::broadcaster::broadcast_actor::BroadcastActorWrapper;
use crate::application::actors::database::db_actor::{DbActor, DbActorAddr};
use crate::application::actors::flight_registry::flight_registry_actor::{
    FlightRegistry, FlightRegistryActorWrapped,
};
use crate::application::actors::iceberg::iceberg_actor::{IcebergActor, IcebergActorAddr};
pub(crate) use crate::application::actors::parser::parser_actor::{ParserActor, ParserActorAddr};

// Test-only mock imports
#[cfg(test)]
use crate::application::actors::database::db_actor::MockDbActor;
#[cfg(test)]
use crate::application::actors::factory::factory_actor::tests::MockFactoryActor;
use crate::application::actors::factory::factory_actor::{FactoryActor, FactoryActorAddr};
#[cfg(test)]
use crate::application::actors::flight_registry::flight_registry_actor::MockFlightRegistry;
#[cfg(test)]
use crate::application::actors::iceberg::iceberg_actor::MockIcebergActor;
use crate::application::actors::messages::registeration::ParserActorReady;
use crate::application::actors::rhai::rhai_actor::{RhaiActorAddr};
use crate::core::error::exception::registry::RegistryError;

// Registry struct
#[derive(Clone)]
pub struct Registry {
    pub db_actor_addr: DbActorAddr,
    pub flight_registry_actor_addr: FlightRegistryActorWrapped,
    pub iceberg_actor_addr: IcebergActorAddr,
    pub iceberg_meter_actor_addr: IcebergActorAddr, // Meter actor
    pub wal_actor_addr: WalActorWrapper,
    pub wal_metric_actor_addr: WalMetricActorWrapper,
    pub factory_actor: FactoryActorAddr,

    // dynamic actors, created 1 per flight
    pub broadcast_actor: HashMap<String, BroadcastActorWrapper>, // This actor is created dynamically based on the arrow stream
    pub parser_actor_addr: HashMap<String, Vec<ParserActorAddr>>, // This actor is created dynamically based on the arrow stream
    pub rhai_actor: HashMap<String, RhaiActorAddr>,
}

// Registry Builder
pub struct RegistryBuilder {
    db_actor_addr: Option<DbActorAddr>,
    flight_registry_actor_addr: Option<FlightRegistryActorWrapped>,
    iceberg_actor_addr: Option<IcebergActorAddr>,
    iceberg_meter_actor_addr: Option<IcebergActorAddr>,
    wal_actor_addr: Option<WalActorWrapper>,
    wal_metric_actor_addr: Option<WalMetricActorWrapper>,
    factory_actor_addr: Option<FactoryActorAddr>,

    // dynamic actors, created 1 per flight
    broadcast_actor: HashMap<String, BroadcastActorWrapper>,
    parser_actor_addr: HashMap<String, Vec<ParserActorAddr>>,
    rhai_actor: HashMap<String, RhaiActorAddr>,
}

#[cfg(test)]
use crate::application::actors::wal::log::log_wal_actor::MockWalActor;
#[cfg(test)]
use crate::application::actors::wal::metric::metric_wal_actor::tests::MockWalMetricActor;

impl RegistryBuilder {
    pub fn new() -> Self {
        Self {
            db_actor_addr: None,
            flight_registry_actor_addr: None,
            iceberg_actor_addr: None,
            iceberg_meter_actor_addr: None,
            wal_actor_addr: None,
            wal_metric_actor_addr: None,
            factory_actor_addr: None,
            broadcast_actor: HashMap::new(),
            parser_actor_addr: HashMap::new(),
            rhai_actor: HashMap::new(),
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

    #[cfg(not(test))]
    pub fn flight_registry_actor(mut self, actor: FlightRegistry) -> Self {
        self.flight_registry_actor_addr = Some(FlightRegistryActorWrapped::Real(actor.start()));
        self
    }

    #[cfg(test)]
    pub fn flight_registry_actor(mut self, actor: MockFlightRegistry) -> Self {
        self.flight_registry_actor_addr = Some(FlightRegistryActorWrapped::Mock(actor.start()));
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
    pub fn iceberg_meter_actor(mut self, meter_actor: IcebergActor) -> Self {
        self.iceberg_meter_actor_addr = Some(IcebergActorAddr::Real(meter_actor.start()));
        self
    }

    #[cfg(test)]
    pub fn iceberg_meter_actor(mut self, meter_actor: MockIcebergActor) -> Self {
        self.iceberg_meter_actor_addr = Some(IcebergActorAddr::Mock(meter_actor.start()));
        self
    }

    #[cfg(not(test))]
    pub fn wal_actor(mut self, actor: WalActor) -> Self {
        self.wal_actor_addr = Some(WalActorWrapper::Real(actor.start()));
        self
    }

    #[cfg(test)]
    pub fn wal_actor(mut self, actor: MockWalActor) -> Self {
        self.wal_actor_addr = Some(WalActorWrapper::Mock(actor.start()));
        self
    }

    #[cfg(not(test))]
    pub fn wal_metric_actor(mut self, actor: WalMetricActor) -> Self {
        self.wal_metric_actor_addr = Some(WalMetricActorWrapper::Real(actor.start()));
        self
    }

    #[cfg(test)]
    pub fn wal_metric_actor(mut self, actor: MockWalMetricActor) -> Self {
        self.wal_metric_actor_addr = Some(WalMetricActorWrapper::Mock(actor.start()));
        self
    }

    #[cfg(not(test))]
    pub fn factory_actor(mut self, actor: FactoryActor) -> Self {
        self.factory_actor_addr = Some(FactoryActorAddr::Real(actor.start()));
        self
    }

    #[cfg(test)]
    pub fn factory_actor(mut self, actor: MockFactoryActor) -> Self {
        self.factory_actor_addr = Some(FactoryActorAddr::Mock(actor.start()));
        self
    }

    pub fn build(self) -> Registry {
        Registry {
            db_actor_addr: self.db_actor_addr.expect("db_actor_addr must be set"),
            flight_registry_actor_addr: self
                .flight_registry_actor_addr
                .expect("flight_registry_actor_addr must be set"),
            iceberg_actor_addr: self
                .iceberg_actor_addr
                .expect("iceberg_actor_addr must be set"),
            iceberg_meter_actor_addr: self
                .iceberg_meter_actor_addr
                .expect("iceberg_meter_actor_addr must be set"),
            wal_actor_addr: self.wal_actor_addr.expect("wal_actor_addr must be set"),
            wal_metric_actor_addr: self
                .wal_metric_actor_addr
                .expect("metric_wal_actor must be set"),
            factory_actor: self
                .factory_actor_addr
                .expect("factory_actor_addr must be set"),
            broadcast_actor: self.broadcast_actor,
            parser_actor_addr: self.parser_actor_addr,
            rhai_actor: self.rhai_actor,
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

#[derive(Message)]
#[rtype(result = "Result<WalActorWrapper, ()>")]
pub struct FetchWalActor;

// Handlers for fetching actors
impl Handler<FetchWalActor> for Registry {
    type Result = Result<WalActorWrapper, ()>;
    fn handle(&mut self, _: FetchWalActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.wal_actor_addr.clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<WalMetricActorWrapper, ()>")]
pub struct FetchWalMetricActor;

// Handlers for fetching actors
impl Handler<FetchWalMetricActor> for Registry {
    type Result = Result<WalMetricActorWrapper, ()>;
    fn handle(&mut self, _: FetchWalMetricActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.wal_metric_actor_addr.clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<FlightRegistryActorWrapped, Error>")]
pub struct FetchFlightRegistryActor;

impl Handler<FetchFlightRegistryActor> for Registry {
    type Result = Result<FlightRegistryActorWrapped, Error>;
    fn handle(&mut self, _: FetchFlightRegistryActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.flight_registry_actor_addr.clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<IcebergActorAddr, ActorError>")]
pub struct FetchIcebergActor;

impl Handler<FetchIcebergActor> for Registry {
    type Result = Result<IcebergActorAddr, ActorError>;
    fn handle(&mut self, _: FetchIcebergActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.iceberg_actor_addr.clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<IcebergActorAddr, ActorError>")]
pub struct FetchIcebergMeterActor;

impl Handler<FetchIcebergMeterActor> for Registry {
    type Result = Result<IcebergActorAddr, ActorError>;
    fn handle(&mut self, _: FetchIcebergMeterActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.iceberg_meter_actor_addr.clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<BroadcastActorWrapper, RegistryError>")]
pub struct FetchBroadcastActor {
    pub flight_name: String,
}

impl Handler<FetchBroadcastActor> for Registry {
    type Result = Result<BroadcastActorWrapper, RegistryError>;
    fn handle(
        &mut self,
        fetch_broadcast_actor: FetchBroadcastActor,
        _: &mut Self::Context,
    ) -> Self::Result {
        self.broadcast_actor
            .get(fetch_broadcast_actor.flight_name.as_str())
            .cloned()
            .ok_or(RegistryError::ActorNotInitialized(
                "Broadcast actor not initialized".to_string(),
            ))
    }
}

#[derive(Message)]
#[rtype(result = "Result<DbActorAddr, ()>")]
pub struct FetchDbActor;
impl Handler<FetchDbActor> for Registry {
    type Result = Result<DbActorAddr, ()>;
    fn handle(&mut self, _: FetchDbActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.db_actor_addr.clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<Vec<ParserActorAddr>, ()>")]
pub struct FetchParserActor {
    pub flight_name: String,
}

impl Handler<FetchParserActor> for Registry {
    type Result = Result<Vec<ParserActorAddr>, ()>;
    fn handle(
        &mut self,
        fetch_parser_actor: FetchParserActor,
        _: &mut Self::Context,
    ) -> Self::Result {
        let flight_name = fetch_parser_actor.flight_name.clone();

        let parsing_actor = self.parser_actor_addr.get(flight_name.as_str());
        if let None = parsing_actor {
            error!("Parser actor not initialized for flight {}", flight_name);
        }
        Ok(parsing_actor.unwrap().clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<FactoryActorAddr, ()>")]
pub struct FetchFactoryActor;

impl Handler<FetchFactoryActor> for Registry {
    type Result = Result<FactoryActorAddr, ()>;
    fn handle(&mut self, _: FetchFactoryActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.factory_actor.clone())
    }
}

/// Registration handlers
/// All the actors register with this actor once they start.
/// TODO - Unregister once stopped.
impl Handler<ParserActorReady> for Registry {
    type Result = ();
    fn handle(&mut self, msg: ParserActorReady, _: &mut Self::Context) -> Self::Result {
        let parsers = self
            .parser_actor_addr
            .entry(msg.flight_name.clone())
            .or_insert_with(Vec::new);

        parsers.push(msg.parser_actor_addr);
    }
}

impl Handler<RhaiActorReady> for Registry {
    type Result = ();

    fn handle(&mut self, msg: RhaiActorReady, _ctx: &mut Self::Context) -> Self::Result {
        self.rhai_actor
            .entry(msg.flight_name.clone())
            .or_insert_with(|| msg.rhai_actor_addr);
    }
}

impl Handler<DbActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: DbActorAddr, _: &mut Self::Context) {
        self.db_actor_addr = msg;
    }
}

impl Handler<FlightRegistryActorWrapped> for Registry {
    type Result = ();
    fn handle(&mut self, msg: FlightRegistryActorWrapped, _: &mut Self::Context) {
        self.flight_registry_actor_addr = msg;
    }
}

impl Handler<IcebergActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: IcebergActorAddr, _: &mut Self::Context) {
        self.iceberg_actor_addr = msg.clone();
        self.iceberg_meter_actor_addr = msg;
    }
}

impl Handler<WalActorWrapper> for Registry {
    type Result = ();
    fn handle(&mut self, msg: WalActorWrapper, _: &mut Self::Context) {
        self.wal_actor_addr = msg;
    }
}

impl Handler<WalMetricActorWrapper> for Registry {
    type Result = ();

    fn handle(&mut self, msg: WalMetricActorWrapper, _: &mut Self::Context) {
        self.wal_metric_actor_addr = msg;
        print!("wal metric actor wrapper:");
        info!("wal matric actor wrapper: {:?}", self.wal_metric_actor_addr);
    }
}

impl Handler<FactoryActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: FactoryActorAddr, _: &mut Self::Context) {
        self.factory_actor = msg;
    }
}

use crate::application::actors::wal::log::log_wal_actor::{WalActor, WalActorWrapper};
use crate::core::error::exception::actor_errors::ActorError;

pub(crate) use crate::application::actors::messages::registeration::RegisterBroadcastActor;
use crate::application::actors::rhai::handler::record_batch_wrapper::RhaiActorReady;
use crate::application::actors::wal::metric::metric_wal_actor::{
    WalMetricActor, WalMetricActorWrapper,
};

impl Handler<RegisterBroadcastActor> for Registry {
    type Result = ();

    fn handle(&mut self, msg: RegisterBroadcastActor, _ctx: &mut Self::Context) -> Self::Result {
        // Correctly handle the initial registration
        self.broadcast_actor
            .insert(msg.flight_name.clone(), msg.broadcast_actor_wrapped_single);
    }
}

#[derive(Message)]
#[rtype(result = "Result<ActorAddr, RegistryError>")]
pub struct FetchActor {
    pub kind: ActorKind,
}

#[derive(Debug, Clone)]
pub enum ActorKind {
    Db,
    FlightRegistry,
    Iceberg,
    Wal,
    Parser,
    Factory,
    Broadcast,
}

#[derive(Debug, Clone)]
pub enum ActorAddr {
    Db(DbActorAddr),
    FlightRegistry(FlightRegistryActorWrapped),
    Iceberg(IcebergActorAddr),
    Wal(WalActorWrapper),
    Parser(ParserActorAddr),
    Factory(FactoryActorAddr),
    Broadcast(BroadcastActorWrapper),
}

#[derive(Message)]
#[rtype(result = "Result<RhaiActorAddr, RegistryError>")]
pub struct FetchRhaiActor {
    pub flight_name: String,
}

impl Handler<FetchRhaiActor> for Registry {
    type Result = Result<RhaiActorAddr, RegistryError>;
    fn handle(&mut self, fetch_rhai_actor: FetchRhaiActor, _: &mut Self::Context) -> Self::Result {
        let flight_name = fetch_rhai_actor.flight_name.clone();
        let rhai_actor = self.rhai_actor.get(flight_name.as_str());
        if let None = rhai_actor {
            error!("Rhai actor not initialized for flight {}", flight_name);
        }
        Ok(rhai_actor.unwrap().clone())
    }
}
