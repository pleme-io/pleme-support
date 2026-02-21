//! # pleme-support
//!
//! Universal support ticket system for Pleme platform services.
//!
//! ## Features
//!
//! - **Support Tickets** - Tickets with priorities, statuses, SLA tracking
//! - **CSAT Scores** - Customer satisfaction tracking (1-5 scale)
//! - **Product Scoping** - Multi-product support (novaskyn, lilitu, thai)
//! - **Dashboard Analytics** - 7 comprehensive metrics views
//! - **GraphQL API** - Queries and mutations for ticket management
//! - **Repository Pattern** - PostgreSQL data access layer
//!
//! ## Usage
//!
//! ### In a Service
//!
//! ```rust,no_run
//! use pleme_support::{SupportRepository, SupportQueries, SupportMutations};
//! use sqlx::PgPool;
//! use std::sync::Arc;
//!
//! # async fn example(db_pool: PgPool) {
//! // Create repository
//! let support_repo = Arc::new(SupportRepository::new(db_pool.clone()));
//!
//! // Add to GraphQL context
//! // context.support_repo = support_repo;
//!
//! // Use in GraphQL schema
//! // Schema::build(QueryRoot, MutationRoot, EmptySubscription)
//! //     .data(support_repo)
//! //     .finish()
//! # }
//! ```
//!
//! ### Models
//!
//! ```rust
//! use pleme_support::{SupportTicket, TicketStatus, TicketPriority, CreateTicketInput};
//! use uuid::Uuid;
//!
//! let input = CreateTicketInput {
//!     customer_id: Uuid::new_v4(),
//!     subject: "Login issue".to_string(),
//!     description: "Cannot log in to account".to_string(),
//!     priority: Some("HIGH".to_string()),
//!     category: Some("authentication".to_string()),
//!     ..Default::default()
//! };
//! ```

pub mod models;
pub mod repository;
pub mod graphql;

// Re-export commonly used types
pub use models::*;
pub use repository::SupportRepository;
pub use graphql::{SupportQueries, SupportMutations};

use thiserror::Error;

/// Support system errors
#[derive(Error, Debug)]
pub enum SupportError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Ticket not found: {0}")]
    TicketNotFound(uuid::Uuid),

    #[error("Message not found: {0}")]
    MessageNotFound(uuid::Uuid),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, SupportError>;
