//! Complete-set application and mounted subscription declarations.

#![allow(clippy::double_must_use)]

use core::{
    any::TypeId,
    pin::Pin,
    task::{Context, Poll},
};
use std::sync::Arc;

use crate::{WorkKey, work::SendOutput};

/// Wake-aware UI-thread source for one local subscription.
pub trait LocalSubscriptionSource<Action> {
    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Action>>;
}

impl<Action, Source> LocalSubscriptionSource<Action> for Source
where
    Source: for<'a, 'b> FnMut(&'a mut Context<'b>) -> Poll<Option<Action>> + Unpin + 'static,
{
    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Action>> {
        self(context)
    }
}

/// Bounded producer-side sink for concrete send-subscription items.
pub struct SendSubscriptionSink<Item> {
    submit: Arc<dyn Fn(Item) -> Result<(), SendSubscriptionSinkError<Item>> + Send + Sync>,
}

/// Exact ownership returned when a send-subscription item is not accepted.
#[must_use]
#[non_exhaustive]
pub enum SendSubscriptionSinkError<Item> {
    /// The generation exists, but its start callback has not committed `Started`.
    NotStarted(Item),
    /// Bounded completion ingress has no free slot.
    Full(Item),
    /// Global runtime scheduling authority is closed.
    Closed(Item),
    /// The exact subscription generation is no longer live.
    Stale(Item),
}

impl<Item> SendSubscriptionSinkError<Item> {
    #[must_use]
    pub fn into_item(self) -> Item {
        match self {
            Self::NotStarted(item) | Self::Full(item) | Self::Closed(item) | Self::Stale(item) => {
                item
            }
        }
    }
}

impl<Item> core::fmt::Debug for SendSubscriptionSinkError<Item> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::NotStarted(_) => "SendSubscriptionSinkError::NotStarted(..)",
            Self::Full(_) => "SendSubscriptionSinkError::Full(..)",
            Self::Closed(_) => "SendSubscriptionSinkError::Closed(..)",
            Self::Stale(_) => "SendSubscriptionSinkError::Stale(..)",
        })
    }
}

/// Result of one nonblocking, exactly-once send-subscription start attempt.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SendSubscriptionStartOutcome {
    Started,
    Unavailable,
    Full,
    Closed,
    Rejected,
}

impl<Item> Clone for SendSubscriptionSink<Item> {
    fn clone(&self) -> Self {
        Self {
            submit: Arc::clone(&self.submit),
        }
    }
}

impl<Item> SendSubscriptionSink<Item> {
    /// Attempts one bounded delivery and returns the exact unaccepted item.
    ///
    /// # Errors
    ///
    /// Returns `NotStarted`, `Full`, `Closed`, or `Stale` with the exact supplied item.
    pub fn try_send(&self, item: Item) -> Result<(), SendSubscriptionSinkError<Item>> {
        (self.submit)(item)
    }

    #[doc(hidden)]
    pub fn __runtime_new(
        submit: impl Fn(Item) -> Result<(), SendSubscriptionSinkError<Item>> + Send + Sync + 'static,
    ) -> Self {
        Self {
            submit: Arc::new(submit),
        }
    }
}

/// Start-once send-capable producer for an ongoing concrete item stream.
pub trait SendSubscriptionSource<Item>: Send + 'static {
    /// Attempts nonblocking producer registration exactly once.
    ///
    /// Long-running producer work must not execute in this UI-thread callback.
    fn start(self: Box<Self>, sink: SendSubscriptionSink<Item>) -> SendSubscriptionStartOutcome;
}

/// Opaque complete desired subscription set for one owner.
#[must_use]
pub struct SubscriptionSet<Action> {
    declarations: Vec<Subscription<Action>>,
}

impl<Action> SubscriptionSet<Action> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            declarations: Vec::new(),
        }
    }

    /// Declares a wake-aware local source polled by the shared local-work budget.
    pub fn local<Source>(&mut self, key: WorkKey, revision: u64, source: Source)
    where
        Source: LocalSubscriptionSource<Action> + 'static,
    {
        self.declarations.push(Subscription {
            key,
            source_type: TypeId::of::<Source>(),
            revision,
            source: SubscriptionSource::Local(Box::pin(source)),
        });
    }

    /// Declares a start-once send producer with a UI-thread item mapper.
    pub fn send<Item, Source, Map>(&mut self, key: WorkKey, revision: u64, source: Source, map: Map)
    where
        Item: Send + 'static,
        Source: SendSubscriptionSource<Item>,
        Map: FnMut(Item) -> Action + 'static,
    {
        let mut map = map;
        self.declarations.push(Subscription {
            key,
            source_type: TypeId::of::<Source>(),
            revision,
            source: SubscriptionSource::Send {
                source: Box::new(SendSourceAdapter::<Source, Item> {
                    source: Some(source),
                    _item: core::marker::PhantomData,
                }),
                map: Box::new(move |item| {
                    item.downcast::<Item>()
                        .map_or_else(|_| unreachable!(), |item| map(*item))
                }),
            },
        });
    }
}

impl<Action> Default for SubscriptionSet<Action> {
    fn default() -> Self {
        Self::new()
    }
}

#[doc(hidden)]
pub struct Subscription<Action> {
    pub key: WorkKey,
    pub source_type: TypeId,
    pub revision: u64,
    pub source: SubscriptionSource<Action>,
}

#[doc(hidden)]
pub enum SubscriptionSource<Action> {
    Local(Pin<Box<dyn LocalSubscriptionSource<Action>>>),
    Send {
        source: Box<dyn ErasedSendSubscriptionSource>,
        map: Box<dyn FnMut(SendOutput) -> Action>,
    },
}

#[doc(hidden)]
pub trait ErasedSendSubscriptionSource: Send {
    fn start(
        self: Box<Self>,
        sink: SendSubscriptionSink<SendOutput>,
    ) -> SendSubscriptionStartOutcome;
}

struct SendSourceAdapter<Source, Item> {
    source: Option<Source>,
    _item: core::marker::PhantomData<fn() -> Item>,
}

impl<Source, Item> ErasedSendSubscriptionSource for SendSourceAdapter<Source, Item>
where
    Item: Send + 'static,
    Source: SendSubscriptionSource<Item>,
{
    fn start(
        mut self: Box<Self>,
        sink: SendSubscriptionSink<SendOutput>,
    ) -> SendSubscriptionStartOutcome {
        let typed_sink = SendSubscriptionSink::__runtime_new(move |item: Item| {
            sink.try_send(Box::new(item)).map_err(|error| {
                let recover = |item: SendOutput| {
                    *item
                        .downcast::<Item>()
                        .unwrap_or_else(|_| unreachable!("send subscription item type is retained"))
                };
                match error {
                    SendSubscriptionSinkError::NotStarted(item) => {
                        SendSubscriptionSinkError::NotStarted(recover(item))
                    }
                    SendSubscriptionSinkError::Full(item) => {
                        SendSubscriptionSinkError::Full(recover(item))
                    }
                    SendSubscriptionSinkError::Closed(item) => {
                        SendSubscriptionSinkError::Closed(recover(item))
                    }
                    SendSubscriptionSinkError::Stale(item) => {
                        SendSubscriptionSinkError::Stale(recover(item))
                    }
                }
            })
        });
        Box::new(
            self.source
                .take()
                .unwrap_or_else(|| unreachable!("producer starts exactly once")),
        )
        .start(typed_sink)
    }
}

#[doc(hidden)]
impl<Action> SubscriptionSet<Action> {
    #[must_use]
    pub fn __runtime_into_declarations(self) -> Vec<Subscription<Action>> {
        self.declarations
    }

    pub fn __runtime_push(&mut self, declaration: Subscription<Action>) {
        self.declarations.push(declaration);
    }
}
