// use crate::config::yaml_reader::Settings;
// use crate::core::db::factory::database_factory::RepositoryProvider;
// use crate::platform::registry::Registry;
// use crate::servers::server::PorosServer;
// use actix::Addr;
// use actix_web::web::ServiceConfig;
// use std::sync::Arc;
// use tokio::sync::oneshot::Sender;
//
// pub struct DatabaseServer {
//     pub actor_registry: Addr<Registry>,
//     pub _shutdown_handler: Option<Sender<()>>, // Hold the sender, else the sender is dropped and the receiver receives a None value and stops the server. // TODO: add the postgres database connection pool
// }
//
// use actix_rt::net::TcpListener;
//
// impl PorosServer for DatabaseServer {
//     type Error = ();
//
//     fn configure_routes(config: &mut ServiceConfig)
//     where
//         Self: Sized,
//     {
//         todo!()
//     }
//
//     async fn bootstrap_server(
//         self,
//         config: &Settings,
//     ) -> Result<
//         (
//             Self,
//             impl Future<Output = Result<(), Self::Error>> + Send,
//             Option<Sender<()>>,
//         ),
//         Self::Error,
//     >
//     where
//         Self: Sized {
//         // unimplemented!()
//         let listener = TcpListener::bind("127.0.0.1:5433").await.unwrap();
//
//     }
//
//     async fn start_server(self, config: &Settings)
//     where
//         Self: Sized,
//     {
//         todo!()
//     }
// }
