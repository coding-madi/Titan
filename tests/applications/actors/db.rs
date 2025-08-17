
/// Testing of actor factory using mocks
struct MockDbActor;

impl Actor for MockDbActor {
    type Context = actix::Context<Self>;
}

struct TestInjestRegistry {
    pub db_actor: Addr<MockDbActor>,
}

impl Handler<ReposReady> for MockDbActor {
    type Result = Result<(), ()>;

    fn handle(&mut self, msg: ReposReady, ctx: &mut Self::Context) -> Self::Result {
        println!("Repo ready called with:");
        Ok(())
    }
}

impl Handler<SaveSchema> for MockDbActor {
    type Result = ();

    fn handle(&mut self, msg: SaveSchema, ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}

impl Handler<GetPatternsForTenant> for MockDbActor {
    type Result = Vec<String>;

    fn handle(&mut self, msg: GetPatternsForTenant, ctx: &mut Self::Context) -> Self::Result {
        todo!()
    }
}