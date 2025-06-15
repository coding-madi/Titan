use actix::{Actor, Context};

pub struct Broadcaster;

impl Broadcaster {
    pub fn new() -> Self {
        Broadcaster {}
    }
}

impl Actor for Broadcaster {
    type Context = Context<Self>;
}
