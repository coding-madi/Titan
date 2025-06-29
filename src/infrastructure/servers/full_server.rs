use crate::config::yaml_reader::Settings;
use crate::core::error::exception::server_error::ServerError;
use crate::infrastructure::servers::injest_server::InjestServer;
use crate::infrastructure::servers::query_server::{QueryServer, block_until_shutdown_signal};
use crate::infrastructure::servers::server::PorosServer;
use actix_web::web::ServiceConfig;
use sqlx::PgPool;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;
use tracing::info;
use tracing::log::error;

pub struct FullServer {
    pub pool: PgPool,
    pub query_server: Option<QueryServer>,
    pub injest_server: Option<InjestServer>,
    pub _injest_server_shutdown_sender: Option<Sender<()>>,
}

impl PorosServer for FullServer {
    type Error = ServerError;

    fn configure_routes(_config: &mut ServiceConfig)
    where
        Self: Sized,
    {
        todo!()
    }

    async fn bootstrap_server(
        self,
        config: &Settings,
    ) -> Result<
        (
            Self,
            impl Future<Output = Result<(), Self::Error>> + Send,
            Option<Sender<()>>,
        ),
        Self::Error,
    >
    where
        Self: Sized,
    {
        let pool = self.pool.clone(); // Clone pool if it needs to be shared

        // Bootstrap InjestServer using its own bootstrap_server method
        // You're updating `self.injest_server` and `self.query_server` directly
        let injest_result = self.injest_server.unwrap().bootstrap_server(config).await;

        let (injest_server_returned, injest_running_fut, injest_shutdown_tx) = match injest_result {
            Ok((server, shutdown_fut, tx_opt)) => {
                // If InjestServer returns itself, capture it
                // Note: The `server` part of the tuple might be `()` or `InjestServer`
                // depending on how InjestServer's `bootstrap_server` is defined.
                // Assuming it returns InjestServer, but if not, adjust.
                (server, shutdown_fut, tx_opt)
            }
            Err(error) => {
                error!("Failed to bootstrap InjestServer: {:?}", error);
                return Err(error); // Return the original ServerError
            }
        };

        // Bootstrap QueryServer using its own bootstrap_server method
        let query_result = self.query_server.unwrap().bootstrap_server(config).await;
        let (query_server_returned, query_running_fut, query_shutdown_tx) = match query_result {
            Ok((server, shutdown_fut, tx_opt)) => {
                // Similar to InjestServer, capture returned QueryServer instance
                (server, shutdown_fut, tx_opt)
            }
            Err(error) => {
                // If QueryServer fails, attempt to shut down InjestServer
                error!("Failed to bootstrap QueryServer: {:?}", error);
                if let Some(tx) = injest_shutdown_tx {
                    let _ = tx.send(()); // Signal InjestServer to shut down
                    info!(
                        "Signaled InjestServer to shut down due to QueryServer bootstrap failure."
                    );
                }
                return Err(error); // Return the original ServerError
            }
        };

        // At this point, self.injest_server and self.query_server have been modified/bootstrapped.
        // We now need to define the combined running future.

        let (full_server_shutdown_tx, full_server_shutdown_rx) = oneshot::channel::<()>();

        // This is the future that represents the combined running state of both servers.
        // It will complete when either sub-server's future completes or a shutdown signal is received.
        let combined_server_future = async move {
            tokio::select! {
                // Await both sub-server futures. If one completes, the `select!` will proceed.
                // We'll primarily wait for the shutdown signal, but also propagate errors if a sub-server crashes.
                injest_res = injest_running_fut => {
                    match injest_res {
                        Ok(_) => info!("InjestServer future completed successfully."),
                        Err(e) => {
                            error!("InjestServer future completed with an error: {:?}", e);
                            // Propagate the error up
                            return Err(e);
                        },
                    }
                }
                query_res = query_running_fut => {
                    match query_res {
                        Ok(_) => info!("QueryServer future completed successfully."),
                        Err(e) => {
                            error!("QueryServer future completed with an error: {:?}", e);
                            // Propagate the error up
                            return Err(e);
                        },
                    }
                }
                _ = full_server_shutdown_rx => {
                    // Received a shutdown signal for the FullServer
                    info!("FullServer received shutdown signal.");

                    // Signal individual servers to shut down gracefully
                    if let Some(tx) = injest_shutdown_tx {
                        let _ = tx.send(());
                        info!("Signaled InjestServer for graceful shutdown.");
                    }
                    if let Some(tx) = query_shutdown_tx {
                        let _ = tx.send(());
                        info!("Signaled QueryServer for graceful shutdown.");
                    }

                    // You might want to await the actual completion of injest_running_fut
                    // and query_running_fut here after sending shutdown signals,
                    // but for a simple `select!`, exiting the select block is enough
                    // if the `stop` calls handle their own awaiting.
                }
            }
            // Ensure all remaining futures complete if they were waiting for their own stops
            Ok(())
        };

        let bootstrapped_full_server = FullServer {
            pool, // Uses the 'pool' that was destructured from the original 'self'
            query_server: Some(query_server_returned),
            injest_server: Some(injest_server_returned),
            _injest_server_shutdown_sender: None,
        };

        // You return `self` (the FullServer instance that was passed in via &mut self)
        // and the `combined_server_future`.
        Ok((
            bootstrapped_full_server, // Return the (potentially modified) FullServer instance
            combined_server_future,
            Some(full_server_shutdown_tx), // Return the sender for the FullServer's shutdown
        ))
    }

    async fn start_server(self, config: &Settings)
    where
        Self: Sized,
    {
        info!("Bootstrapping FullServer...");
        let (_server_instance, server_run_future, shutdown_tx_opt) = self
            .bootstrap_server(config)
            .await
            .expect("Failed to bootstrap FullServer"); // Handle errors gracefully in a real app

        // Spawn a task to listen for OS signals (Ctrl+C, SIGTERM)
        tokio::spawn(async move {
            // Your `block_until_shutdown_signal` function
            block_until_shutdown_signal().await;
            info!("OS shutdown signal received, initiating FullServer shutdown.");

            // Use the sender returned by bootstrap_server to signal the main combined future
            if let Some(tx) = shutdown_tx_opt {
                let _ = tx.send(());
            } else {
                error!("FullServer shutdown sender not available!");
            }
        });

        info!("FullServer is now running. Awaiting combined server future...");
        // Await the future that represents the running state of both sub-servers.
        // This line keeps your `start_server` method (and thus your application) alive.
        server_run_future
            .await
            .expect("FullServer run future failed"); // Handle errors gracefully

        info!("FullServer has stopped.");
    }
}
