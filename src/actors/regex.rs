use actix::{Actor, Context, Handler};

use crate::actors::broadcast::{RecordBatchWrapper, RegexRule};

pub struct RegexActor {
    pub regex: String,
}


impl Actor for RegexActor {
    type Context = Context<Self>;
}

impl Handler<RegexRule> for RegexActor {
    type Result = Result<(), String>;

    fn handle(&mut self, msg: RegexRule, _ctx: &mut Self::Context) -> Self::Result {
        // Here we would typically compile the regex and store it in the actor's state
        // For now, we just print the message
        println!("Received RegexRule: {:?}", msg);
        
        // Simulate some processing with the regex
        self.regex = msg.pattern; // Store the regex pattern in the actor's state
        
        Ok(())
    }
}

impl Handler<RecordBatchWrapper> for RegexActor {
    type Result = ();

    fn handle(&mut self, _msg: RecordBatchWrapper, _ctx: &mut Self::Context) -> Self::Result {
        // Here we would typically apply the regex to the data in the RecordBatchWrapper
        // For now, we just print the message
        println!("Received RecordBatchWrapper");
        
        // Simulate processing the record batch with the regex
        // This is where you would apply the regex to the data
    }
}