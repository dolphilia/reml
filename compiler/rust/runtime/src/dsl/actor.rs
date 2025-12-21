//! Core.Dsl.Actor の最小実装。

use std::sync::Arc;

use crate::runtime::async_bridge::ActorSystem;

/// アクター定義。
#[derive(Clone)]
pub struct ActorDefinition<Message> {
    pub name: String,
    pub on_message: Arc<dyn Fn(Message) -> ActorResult<()> + Send + Sync>,
}

impl<Message> ActorDefinition<Message> {
    pub fn new(
        name: impl Into<String>,
        on_message: Arc<dyn Fn(Message) -> ActorResult<()> + Send + Sync>,
    ) -> Self {
        Self {
            name: name.into(),
            on_message,
        }
    }

    pub fn handle(&self, message: Message) -> ActorResult<()> {
        (self.on_message)(message)
    }
}

/// DSL から利用するメールボックス。
#[derive(Clone)]
pub struct MailboxBridge<Message> {
    send_fn: Arc<dyn Fn(Message) -> ActorResult<()> + Send + Sync>,
    recv_fn: Arc<dyn Fn() -> ActorResult<Message> + Send + Sync>,
}

impl<Message> MailboxBridge<Message> {
    pub fn new(
        send_fn: Arc<dyn Fn(Message) -> ActorResult<()> + Send + Sync>,
        recv_fn: Arc<dyn Fn() -> ActorResult<Message> + Send + Sync>,
    ) -> Self {
        Self { send_fn, recv_fn }
    }

    pub fn send(&self, message: Message) -> ActorResult<()> {
        (self.send_fn)(message)
    }

    pub fn receive(&self) -> ActorResult<Message> {
        (self.recv_fn)()
    }
}

/// 監督仕様。
#[derive(Debug, Clone)]
pub struct SupervisorSpec {
    pub label: String,
    pub max_restarts: u32,
}

impl SupervisorSpec {
    pub fn new(label: impl Into<String>, max_restarts: u32) -> Self {
        Self {
            label: label.into(),
            max_restarts,
        }
    }
}

/// 監督ブリッジ。
#[derive(Debug, Clone)]
pub struct SupervisionBridge {
    pub spec: SupervisorSpec,
}

/// アクターエラー。
#[derive(Debug, Clone)]
pub struct ActorError {
    pub kind: ActorErrorKind,
    pub message: String,
}

impl ActorError {
    pub fn new(kind: ActorErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// アクターエラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActorErrorKind {
    SpawnFailed,
    MailboxUnavailable,
    RuntimeFailure,
}

pub type ActorResult<T> = Result<T, ActorError>;

/// Core.Dsl.Actor の名前空間。
pub struct Actor;

impl Actor {
    pub fn spawn<Message>(
        system: &ActorSystem<Message>,
        def: ActorDefinition<Message>,
        supervision: Option<SupervisionBridge>,
    ) -> ActorResult<MailboxBridge<Message>> {
        system.spawn(def, supervision)
    }
}
