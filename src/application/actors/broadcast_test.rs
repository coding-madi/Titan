#[cfg(test)]
pub mod test {
    use crate::api::http::regex::RegexRequest;
    use crate::application::actors::broadcast::{BroadcastActorAddr, MockBroadcastActor};
    use crate::application::actors::db::{DbActorAddr, MockDbActor};
    use crate::application::actors::flight_registry::{
        FlightRegistryActorAddr, MockFlightRegistry,
    };
    use crate::application::actors::iceberg::{IcebergActorAddr, MockIcebergActor};
    use crate::application::actors::parser::MockParsingActor;
    use crate::application::actors::wal::{MockWalActor, WalActorAddr};
    use crate::platform::registry::{FetchParserActor, ParserActorAddr, Registry, RegistryBuilder};
    use actix::Actor;
    use actix_web::web::to;
    use futures_util::SinkExt;
    use std::thread::spawn;
    use std::time::Duration;
    use tokio::time::sleep;
    use tracing_subscriber::registry;

    #[actix_web::test]
    pub async fn test_broadcast() {
        let registry = Registry {
            db_actor_addr: DbActorAddr::Empty,
            broadcast_actor_addr: BroadcastActorAddr::Empty,
            flight_registry_actor_addr: FlightRegistryActorAddr::Empty,
            iceberg_actor_addr: IcebergActorAddr::Empty,
            parser_actor_addr: ParserActorAddr::Empty,
            wal_actor_addr: WalActorAddr::Empty,
        };

        let registry_address = registry.start();
        let db_actor = MockDbActor {
            registry_address: registry_address.clone(),
        };
        let broadcast_actor = MockBroadcastActor {
            registry_address: registry_address.clone(),
        };
        let flight_registry_actor = MockFlightRegistry {
            registry_address: registry_address.clone(),
        };
        let iceberg_actor = MockIcebergActor {
            registry_address: registry_address.clone(),
        };
        let parser_actor = vec![MockParsingActor {
            registry_address: registry_address.clone(),
        }];
        let wal_actor = MockWalActor::new(registry_address.clone());

        let registry = RegistryBuilder::new()
            .broadcast_actor(broadcast_actor)
            .db_actor(db_actor)
            .flight_registry_actor(flight_registry_actor)
            .iceberg_actor(iceberg_actor)
            .parser_actor(parser_actor)
            .wal_actor(wal_actor)
            .build();

        let mut retries = 0;
        let max_retries = 10;
        let delay_ms = 50; // Milliseconds to wait between retries

        let mut registered_addr: Option<BroadcastActorAddr> = None;

        while retries < max_retries {
            let parsed_actor = registry_address
                .send(FetchParserActor)
                .await
                .expect("TODO: panic message")
                .expect("TODO: panic message");
            let ParserActorAddr::Mock(actor) = parsed_actor else {
                println!("Not mock actor");
                continue;
            };

            for i in actor {
                i.send(RegexRequest {
                    name: "".to_string(),
                    tenant: "".to_string(),
                    flight_id: "".to_string(),
                    log_group: "".to_string(),
                    pattern: vec![],
                })
                .await;

                let x = registry.clone();
            }
            retries += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
        }
        assert!(registered_addr.is_some(), "BroadcastActor address was not registered in Registry after retries.");
    }
}
