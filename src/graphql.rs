//! GraphQL API for support ticket system
//!
//! Provides SupportQueries and SupportMutations that can be integrated
//! into any service's GraphQL schema.
//!
//! ## Usage in Services
//!
//! Services should delegate to these query/mutation structs and provide
//! SupportRepository in the GraphQL context.
//!
//! Authorization checks should be done by the service layer before
//! delegating to these resolvers.

use async_graphql::{Context, Object, Result as GraphQLResult};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::models::{
    SupportTicket, TicketMessage, CreateTicketInput, UpdateTicketInput,
    AddTicketMessageInput, TicketFilter, CrmCoreSupportDashboardMetrics,
};
use crate::repository::SupportRepository;

pub struct SupportQueries;

#[Object(name = "Query", extends)]
impl SupportQueries {
    /// Get a single support ticket by ID
    ///
    /// Note: Services should implement authorization checks before calling this
    async fn support_ticket(&self, ctx: &Context<'_>, id: Uuid) -> GraphQLResult<SupportTicket> {
        let support_repo = ctx.data::<Arc<SupportRepository>>()?;

        let ticket = support_repo.find_by_id(id).await?;
        Ok(ticket)
    }

    /// List support tickets with filters
    ///
    /// Note: Services should implement authorization checks and apply filters
    async fn support_tickets(
        &self,
        ctx: &Context<'_>,
        product: String,
        filter: Option<TicketFilter>,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> GraphQLResult<Vec<SupportTicket>> {
        let support_repo = ctx.data::<Arc<SupportRepository>>()?;

        let filter = filter.unwrap_or_else(|| TicketFilter {
            status: None,
            priority: None,
            assigned_to: None,
            customer_id: None,
            category: None,
            search_query: None,
        });

        let tickets = support_repo.list(
            &product,
            &filter,
            limit.unwrap_or(20),
            offset.unwrap_or(0),
        ).await?;

        Ok(tickets)
    }

    /// Get messages for a ticket
    async fn ticket_messages(&self, ctx: &Context<'_>, ticket_id: Uuid) -> GraphQLResult<Vec<TicketMessage>> {
        let support_repo = ctx.data::<Arc<SupportRepository>>()?;

        let messages = support_repo.get_messages(ticket_id).await?;
        Ok(messages)
    }

    /// Get support dashboard metrics for analytics
    ///
    /// Note: Services should implement admin-only authorization before calling this
    async fn support_dashboard_metrics(
        &self,
        ctx: &Context<'_>,
        product: String,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> GraphQLResult<CrmCoreSupportDashboardMetrics> {
        let support_repo = ctx.data::<Arc<SupportRepository>>()?;

        let metrics = support_repo.get_dashboard_metrics(
            &product,
            period_start,
            period_end,
        ).await?;

        Ok(metrics)
    }
}

pub struct SupportMutations;

#[Object(name = "Mutation", extends)]
impl SupportMutations {
    /// Create a new support ticket
    ///
    /// Note: Services should verify user authentication before calling this
    async fn create_support_ticket(
        &self,
        ctx: &Context<'_>,
        product: String,
        input: CreateTicketInput,
    ) -> GraphQLResult<SupportTicket> {
        let support_repo = ctx.data::<Arc<SupportRepository>>()?;

        let ticket = support_repo.create_ticket(&product, &input).await?;
        Ok(ticket)
    }

    /// Update a support ticket
    ///
    /// Note: Services should implement authorization checks (e.g., support:write permission)
    async fn update_support_ticket(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
        input: UpdateTicketInput,
    ) -> GraphQLResult<SupportTicket> {
        let support_repo = ctx.data::<Arc<SupportRepository>>()?;

        let ticket = support_repo.update_ticket(id, &input).await?;
        Ok(ticket)
    }

    /// Add a message to a ticket
    ///
    /// Note: Services should provide author_id from authenticated user context
    async fn add_ticket_message(
        &self,
        ctx: &Context<'_>,
        author_id: Uuid,
        input: AddTicketMessageInput,
    ) -> GraphQLResult<TicketMessage> {
        let support_repo = ctx.data::<Arc<SupportRepository>>()?;

        let message = support_repo.add_message(author_id, &input).await?;
        Ok(message)
    }
}
