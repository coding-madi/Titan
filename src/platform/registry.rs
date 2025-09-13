use actix::{Actor, Handler, Message};
use std::io::Error;
use tracing::error;
use tracing::log::trace;

// Core Actor imports
use crate::application::actors::broadcast_actor::BroadcastActorWrapper;
use crate::application::actors::db_actor::{DbActor, DbActorAddr};
use crate::application::actors::flight_registry_actor::{
    FlightRegistry, FlightRegistryActorWrapped,
};
use crate::application::actors::iceberg_actor::{IcebergActor, IcebergActorAddr};
pub(crate) use crate::application::actors::parser_actor::{ParserActor, ParserActorAddr};
use crate::application::actors::wal_actor::{WalActor, WalActorWrapper};

// Test-only mock imports
#[cfg(test)]
use crate::application::actors::broadcast_actor::MockBroadcastActor;
#[cfg(test)]
use crate::application::actors::db_actor::MockDbActor;
#[cfg(test)]
use crate::application::actors::factory::factory_actor::tests::MockFactoryActor;
use crate::application::actors::factory::factory_actor::{FactoryActor, FactoryActorAddr};
#[cfg(test)]
use crate::application::actors::flight_registry_actor::MockFlightRegistry;
#[cfg(test)]
use crate::application::actors::iceberg_actor::MockIcebergActor;
#[cfg(test)]
use crate::application::actors::parser_actor::MockParsingActor;
#[cfg(test)]
use crate::application::actors::wal_actor::MockWalActor;
use crate::core::error::exception::registry::RegistryError;

// Registry struct
#[derive(Clone)]
pub struct Registry {
    pub db_actor_addr: DbActorAddr,
    pub flight_registry_actor_addr: FlightRegistryActorWrapped,
    pub iceberg_actor_addr: IcebergActorAddr,
    pub wal_actor_addr: WalActorWrapper,
    pub parser_actor_addr: Option<ParserActorAddr>, // This actor is created dynamically based on the arrow stream
    pub factory_actor: FactoryActorAddr,
    pub broadcast_actor: Option<BroadcastActorWrapper>, // This actor is created dynamically based on the arrow stream
}

// Registry Builder
pub struct RegistryBuilder {
    db_actor_addr: Option<DbActorAddr>,
    flight_registry_actor_addr: Option<FlightRegistryActorWrapped>,
    iceberg_actor_addr: Option<IcebergActorAddr>,
    wal_actor_addr: Option<WalActorWrapper>,
    parser_actor_addr: Option<ParserActorAddr>,
    factory_actor_addr: Option<FactoryActorAddr>,
    broadcast_actor: Option<BroadcastActorWrapper>,
}

impl RegistryBuilder {
    pub fn new() -> Self {
        Self {
            db_actor_addr: None,
            flight_registry_actor_addr: None,
            iceberg_actor_addr: None,
            wal_actor_addr: None,
            parser_actor_addr: None,
            factory_actor_addr: None,
            broadcast_actor: None,
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
            wal_actor_addr: self.wal_actor_addr.expect("wal_actor_addr must be set"),
            parser_actor_addr: self.parser_actor_addr,
            factory_actor: self
                .factory_actor_addr
                .expect("factory_actor_addr must be set"),
            broadcast_actor: self.broadcast_actor,
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
#[rtype(result = "Result<FlightRegistryActorWrapped, Error>")]
pub struct FetchFlightRegistryActor;

impl Handler<FetchFlightRegistryActor> for Registry {
    type Result = Result<FlightRegistryActorWrapped, Error>;
    fn handle(&mut self, _: FetchFlightRegistryActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.flight_registry_actor_addr.clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<IcebergActorAddr, ()>")]
pub struct FetchIcebergActor;

impl Handler<FetchIcebergActor> for Registry {
    type Result = Result<IcebergActorAddr, ()>;
    fn handle(&mut self, _: FetchIcebergActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.iceberg_actor_addr.clone())
    }
}

#[derive(Message)]
#[rtype(result = "Result<BroadcastActorWrapper, RegistryError>")]
pub struct FetchBroadcastActor {
    pub flight_name: String,
}

impl Handler<FetchBroadcastActor> for Registry {
    type Result = Result<BroadcastActorWrapper, RegistryError>;
    fn handle(&mut self, _: FetchBroadcastActor, _: &mut Self::Context) -> Self::Result {
        self.broadcast_actor
            .clone()
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
#[rtype(result = "Result<ParserActorAddr, ()>")]
pub struct FetchParserActor {
    pub flight_name: String,
}

impl Handler<FetchParserActor> for Registry {
    type Result = Result<ParserActorAddr, ()>;
    fn handle(&mut self, _: FetchParserActor, _: &mut Self::Context) -> Self::Result {
        Ok(self.parser_actor_addr.clone().unwrap())
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
impl Handler<ParserActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: ParserActorAddr, _: &mut Self::Context) -> Self::Result {
        self.parser_actor_addr = Some(msg);
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
        self.iceberg_actor_addr = msg;
    }
}

impl Handler<WalActorWrapper> for Registry {
    type Result = ();
    fn handle(&mut self, msg: WalActorWrapper, _: &mut Self::Context) {
        self.wal_actor_addr = msg;
    }
}

impl Handler<FactoryActorAddr> for Registry {
    type Result = ();
    fn handle(&mut self, msg: FactoryActorAddr, _: &mut Self::Context) {
        self.factory_actor = msg;
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct RegisterBroadcastActor {
    pub broadcast_actor_wrapped_single: BroadcastActorWrapper,
}

impl Handler<RegisterBroadcastActor> for Registry {
    type Result = ();

    fn handle(&mut self, msg: RegisterBroadcastActor, ctx: &mut Self::Context) -> Self::Result {
        // Correctly handle the initial registration
        if self.broadcast_actor.is_none() {
            self.broadcast_actor = Some(msg.broadcast_actor_wrapped_single);
            println!("Broadcast actor registered for the first time");
            return;
        }

        // Get a mutable reference to the actor's state
        let current_actor_wrapped = self.broadcast_actor.as_mut().unwrap();

        // Merge the new actor into the existing one directly
        let result =
            merge_broadcast_actors(current_actor_wrapped, msg.broadcast_actor_wrapped_single);

        match result {
            Ok(_) => {
                println!("Successfully merged new broadcast actor.");
            }
            Err(e) => {
                error!("Error in merging broadcast actors: {}", e);
            }
        }
    }
}

fn merge_broadcast_actors(
    current: &BroadcastActorWrapper,
    new: BroadcastActorWrapper,
) -> Result<BroadcastActorWrapper, RegistryError> {
    match (current.clone(), &new.clone()) {
        (BroadcastActorWrapper::Real(mut current_map), BroadcastActorWrapper::Real(new)) => {
            let flight = new.keys().next().unwrap();
            if current_map.contains_key(flight) {
                error!("Broadcast actor already exists for flight {}", flight);
            } else {
                current_map.insert(flight.clone(), new.get(flight).unwrap().clone());
            }
            Ok(BroadcastActorWrapper::Real(current_map))
        }
        #[cfg(test)]
        (BroadcastActorWrapper::Mock(mut current), BroadcastActorWrapper::Mock(new)) => {
            let flight = new.keys().next().unwrap();
            if current.contains_key(flight) {
                error!("Broadcast actor already exists for flight {}", flight);
            } else {
                current.insert(flight.clone(), new.get(flight).unwrap().clone());
            }
            Ok(BroadcastActorWrapper::Mock(current))
        }
        _ => Err(RegistryError::ActorNotInitialized(
            "Error in actor types".to_string(),
        )),
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
