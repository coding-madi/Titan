pub mod test {
    use crate::application::actors::broadcast::RecordBatchWrapper;
    use crate::platform::registry::Registry;
    use actix::{Actor, Addr, Context, Handler};
    use tracing::info;

    pub struct MockWalActor {}

    impl MockWalActor {
        pub fn new(registry_address: Addr<Registry>) -> Self {
            MockWalActor {}
        }
    }

    impl Actor for MockWalActor {
        type Context = Context<Self>;

        fn started(&mut self, _ctx: &mut Self::Context) {
            info!("Mock WAL actor started");
        }
    }

    impl Handler<RecordBatchWrapper> for MockWalActor {
        type Result = ();
        fn handle(&mut self, _record_batch_wrapper: RecordBatchWrapper, _ctx: &mut Self::Context) {
            info!("Mock WAL actor received message");
        }
    }
}
