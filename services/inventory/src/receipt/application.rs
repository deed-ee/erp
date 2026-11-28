//! Connect authorized preview receiving to its owner and persistence.
//! Test-gated; no public receipt routes or actor grants are provisioned here.

use super::{
    Command, Preparation, PurchaseLine, Reject, Selection, StockKey,
    mongo::{self, ActorKind, Intent, Post, Prepare, Store},
    source::{Receiver, Selected},
};
use axum::http::{HeaderMap, StatusCode};
use erp4us_identity::Auth;
use tokio::net::ToSocketAddrs;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Failure {
    Request(Reject),
    Storage(mongo::Failure),
    Owner(StatusCode),
}

impl From<mongo::Failure> for Failure {
    fn from(error: mongo::Failure) -> Self {
        Self::Storage(error)
    }
}

#[derive(Clone, Copy)]
pub(super) struct Line {
    pub order: Uuid,
    pub line: Uuid,
    pub purchase: PurchaseLine,
}

pub(super) struct Receiving<'a> {
    store: &'a Store,
    receiver: Receiver,
}

impl<'a> Receiving<'a> {
    pub(super) fn authorize(
        store: &'a Store,
        auth: &Auth,
        headers: &HeaderMap,
    ) -> Result<Self, StatusCode> {
        Ok(Self {
            store,
            receiver: Receiver::verify(auth, headers)?,
        })
    }

    fn selection(&self, line: Line) -> Result<Selection, Failure> {
        Selection::new(self.receiver.scope(), line.order, line.line, line.purchase)
            .map_err(Failure::Request)
    }

    pub(super) async fn reopen(&self, line: Line) -> Result<Option<Intent>, Failure> {
        self.store
            .reopen(self.selection(line)?)
            .await
            .map_err(Failure::from)
    }

    pub(super) async fn prepare(
        &self,
        line: Line,
        acknowledged: Option<Uuid>,
        pieces: &str,
        peer: impl ToSocketAddrs,
    ) -> Result<Prepare, Failure> {
        let selection = self.selection(line)?;
        let request =
            Preparation::new(selection, acknowledged, pieces).map_err(Failure::Request)?;
        if let Some(current) = self.store.prepare_replay(request).await? {
            return Ok(Prepare::Replay(current));
        }
        let (source, destination) = self.source(selection, peer).await?;
        self.store
            .prepare(request, &source.as_issued(), &destination)
            .await
            .map_err(Failure::from)
    }

    pub(super) async fn confirm(
        &self,
        line: Line,
        receipt: Uuid,
        pieces: &str,
        peer: impl ToSocketAddrs,
    ) -> Result<Post, Failure> {
        let command = Command::new(
            self.receiver.scope(),
            line.order,
            line.line,
            line.purchase,
            receipt,
            pieces,
        )
        .map_err(Failure::Request)?;
        if let Some(result) = self.store.replay(command).await? {
            return Ok(Post::Replay(result));
        }
        let (source, destination) = self.source(command.selection(), peer).await?;
        self.store
            .post(command, &source.as_issued(), &destination, ActorKind::User)
            .await
            .map_err(Failure::from)
    }

    async fn source(
        &self,
        selection: Selection,
        peer: impl ToSocketAddrs,
    ) -> Result<(Selected, StockKey), Failure> {
        let (plan, warehouse) = self.store.plan_destination(selection.sales).await?;
        let source = self
            .receiver
            .fetch(peer, plan, selection.purchase)
            .await
            .map_err(Failure::Owner)?;
        let destination = StockKey {
            scope: selection.sales.scope,
            warehouse,
            item: source.as_issued().item.clone(),
        };
        Ok((source, destination))
    }
}
