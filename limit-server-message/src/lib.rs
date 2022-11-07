#![feature(type_alias_impl_trait)]

use std::collections::BTreeMap;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc::{Receiver, Sender};
use volo_gen::limit::message::{Message, ReceiveMessagesRequest};
use volo_grpc::{BoxStream, Request, Response, Status};

// require db
// require background worker
// default channel size 100
pub struct MessageService {
    users: BTreeMap<String, (Sender<Message>, Receiver<Message>)>,
}

#[volo::async_trait]
impl volo_gen::limit::message::MessageService for MessageService {
    async fn receive_messages(
        &self,
        req: Request<ReceiveMessagesRequest>,
    ) -> Result<Response<BoxStream<'static, Result<Message, Status>>>, Status> {
        Ok(Response::new(Box::pin(
            self.users[req.get_ref().token.unwrap().jwt].1,
        )))
    }

    async fn send_message(&self, req: Request<Message>) -> Result<Response<Message>, Status> {
        todo!()
    }
}
