//! Core.Async と Core.Dsl.Actor の接続点。

use std::sync::Arc;

use crate::dsl::actor::{ActorDefinition, ActorError, MailboxBridge, SupervisionBridge};

/// Core.Async 互換のアクターシステム。
#[derive(Clone)]
pub struct ActorSystem<Message> {
    spawner: Arc<dyn Fn(ActorDefinition<Message>, Option<SupervisionBridge>) -> ActorResult<MailboxBridge<Message>> + Send + Sync>,
}

impl<Message> ActorSystem<Message> {
    pub fn new(
        spawner: Arc<dyn Fn(ActorDefinition<Message>, Option<SupervisionBridge>) -> ActorResult<MailboxBridge<Message>> + Send + Sync>,
    ) -> Self {
        Self { spawner }
    }

    pub fn spawn(
        &self,
        def: ActorDefinition<Message>,
        supervision: Option<SupervisionBridge>,
    ) -> ActorResult<MailboxBridge<Message>> {
        (self.spawner)(def, supervision)
    }
}

pub type ActorResult<T> = Result<T, ActorError>;
