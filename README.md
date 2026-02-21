# pleme-support

Universal support ticket system for the Pleme platform.

## Features

- **Support Tickets** - Complete ticket management with priorities and statuses
- **SLA Tracking** - First response time, resolution time, breach tracking
- **CSAT Scores** - Customer satisfaction ratings (1-5 scale)
- **Multi-Product** - Product-scoped support (novaskyn, lilitu, thai)
- **Dashboard Analytics** - 7 comprehensive metrics views
- **GraphQL API** - Ready-to-use queries and mutations
- **Repository Pattern** - PostgreSQL data access layer

## Installation

Add to your service's `Cargo.toml`:

```toml
[dependencies]
pleme-support = { path = "../../../../../libraries/rust/crates/pleme-support" }
```

## Database Migration

Run the SQL migration to create tables:

```bash
psql -f migrations/001_support_ticketing_system.sql
```

Or use sqlx migrations in your service.

## Usage in Services

### 1. Create Repository

```rust
use pleme_support::SupportRepository;
use sqlx::PgPool;
use std::sync::Arc;

let support_repo = Arc::new(SupportRepository::new(db_pool.clone()));
```

### 2. Add to GraphQL Context

```rust
// In your GraphQL context
pub struct GraphQLContext {
    pub support_repo: Arc<SupportRepository>,
    // ... other fields
}
```

### 3. Integrate GraphQL API

```rust
use pleme_support::{SupportQueries, SupportMutations};

// In your QueryRoot
async fn support_ticket(&self, ctx: &Context<'_>, id: Uuid) -> GraphQLResult<SupportTicket> {
    // Add authorization check here
    let gql_ctx = ctx.data::<GraphQLContext>()?;
    gql_ctx.require_permission("support:read")?;

    // Delegate to library
    SupportQueries.support_ticket(ctx, id).await
}

// In your MutationRoot
async fn create_support_ticket(
    &self,
    ctx: &Context<'_>,
    product: String,
    input: CreateTicketInput,
) -> GraphQLResult<SupportTicket> {
    // Add authorization check here
    let gql_ctx = ctx.data::<GraphQLContext>()?;
    gql_ctx.require_authenticated()?;

    // Delegate to library
    SupportMutations.create_support_ticket(ctx, product, input).await
}
```

### 4. Provide Repository in GraphQL Execution

```rust
schema
    .execute(request.data(support_repo.clone()))
    .await
```

## Models

### SupportTicket

```rust
use pleme_support::{SupportTicket, TicketStatus, TicketPriority};

pub struct SupportTicket {
    pub id: Uuid,
    pub product: String,
    pub customer_id: Uuid,
    pub subject: String,
    pub description: String,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub assigned_to: Option<Uuid>,
    pub first_response_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub sla_breach: bool,
    pub csat_score: Option<i32>,
    // ... more fields
}
```

### Enums

- `TicketStatus`: NEW, IN_PROGRESS, WAITING_ON_CUSTOMER, RESOLVED, CLOSED
- `TicketPriority`: LOW, MEDIUM, HIGH, URGENT

## GraphQL API

### Queries

- `supportTicket(id: UUID!): SupportTicket`
- `supportTickets(product: String!, filter: TicketFilter, limit: Int, offset: Int): [SupportTicket!]!`
- `ticketMessages(ticketId: UUID!): [TicketMessage!]!`
- `supportDashboardMetrics(product: String!, periodStart: DateTime!, periodEnd: DateTime!): SupportDashboardMetrics`

### Mutations

- `createSupportTicket(product: String!, input: CreateTicketInput!): SupportTicket`
- `updateSupportTicket(id: UUID!, input: UpdateTicketInput!): SupportTicket`
- `addTicketMessage(authorId: UUID!, input: AddTicketMessageInput!): TicketMessage`

## Dashboard Metrics

The `supportDashboardMetrics` query returns comprehensive analytics:

- **Overview**: Total tickets, open, resolved, avg resolution time
- **Status Breakdown**: Counts by status
- **Priority Breakdown**: Counts by priority
- **SLA Metrics**: Breach rate, avg first response time, avg resolution time
- **Response Metrics**: Tickets with/without first response
- **Agent Performance**: Top agents by resolved tickets
- **Trends**: Ticket creation over time

## Authorization

The library is authorization-agnostic. Services MUST implement their own permission checks:

```rust
// Example authorization wrapper
async fn support_ticket(&self, ctx: &Context<'_>, id: Uuid) -> GraphQLResult<SupportTicket> {
    let gql_ctx = ctx.data::<GraphQLContext>()?;

    // Your authorization logic
    gql_ctx.require_permission("support:read")?;

    // Delegate to library
    pleme_support::SupportQueries.support_ticket(ctx, id).await
}
```

## Product Scoping

All queries are product-scoped. Always provide the product parameter:

```rust
support_repo.create_ticket("novaskyn", &input).await?;
support_repo.list("lilitu", &filter, limit, offset).await?;
```

## License

UNLICENSED - Internal Pleme platform use only
