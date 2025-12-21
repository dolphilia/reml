//! Core.Async と Core.Dsl.Actor の接続点。

use std::sync::Arc;

use crate::dsl::actor::{ActorDefinition, ActorError, ActorErrorKind, MailboxBridge, SupervisionBridge};

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

impl<Message> ActorSystem<Message>
where
    Message: Clone + Send + 'static,
{
    /// 最小の in-memory 実装。
    pub fn in_memory() -> Self {
        let spawner = Arc::new(
            move |def: ActorDefinition<Message>,
                  _supervision: Option<SupervisionBridge>|
                  -> ActorResult<MailboxBridge<Message>> {
                let (tx, rx) = std::sync::mpsc::channel::<Message>();
                let def = Arc::new(def);
                let send_def = Arc::clone(&def);
                let send_tx = tx.clone();
                let send_fn = Arc::new(move |message: Message| -> ActorResult<()> {
                    let copy = message.clone();
                    send_def.handle(message)?;
                    send_tx.send(copy).map_err(|_| {
                        ActorError::new(
                            ActorErrorKind::MailboxUnavailable,
                            "actor mailbox unavailable",
                        )
                    })
                });
                let recv_fn = Arc::new(move || -> ActorResult<Message> {
                    rx.recv().map_err(|_| {
                        ActorError::new(
                            ActorErrorKind::MailboxUnavailable,
                            "actor mailbox unavailable",
                        )
                    })
                });
                Ok(MailboxBridge::new(send_fn, recv_fn))
            },
        );
        ActorSystem::new(spawner)
    }
}

pub type ActorResult<T> = Result<T, ActorError>;
