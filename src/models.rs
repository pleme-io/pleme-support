use async_graphql::{SimpleObject, InputObject, Enum};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, SimpleObject)]
pub struct SupportTicket {
    pub id: Uuid,
    pub product: String,
    pub customer_id: Uuid,
    pub subject: String,
    pub description: String,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub category: Option<String>,
    pub assigned_to: Option<Uuid>,
    pub first_response_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub sla_breach: bool,
    pub csat_score: Option<i32>,
    #[graphql(skip)]
    pub metadata: sqlx::types::JsonValue,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Enum, Eq, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "ticket_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TicketStatus {
    New,
    InProgress,
    WaitingOnCustomer,
    Resolved,
    Closed,
}

#[derive(Debug, Clone, Copy, Enum, Eq, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "ticket_priority", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, SimpleObject)]
pub struct TicketMessage {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub author_id: Uuid,
    pub is_internal: bool,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

// Dashboard metrics structures (prefixed with CrmCore to avoid federation conflicts)
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "CrmCoreSupportDashboardMetrics")]
pub struct CrmCoreSupportDashboardMetrics {
    pub overview: CrmCoreSupportOverviewMetrics,
    pub ticket_by_status: Vec<CrmCoreTicketStatusCount>,
    pub ticket_by_priority: Vec<CrmCoreTicketPriorityCount>,
    pub sla_metrics: CrmCoreSlaMetrics,
    pub response_metrics: CrmCoreResponseMetrics,
    pub top_agents: Vec<CrmCoreAgentPerformance>,
    pub ticket_trends: Vec<CrmCoreTicketTrend>,
}

#[derive(Debug, Clone, FromRow, SimpleObject)]
#[graphql(name = "CrmCoreSupportOverviewMetrics")]
pub struct CrmCoreSupportOverviewMetrics {
    pub total_active_tickets: i64,
    pub new_tickets_today: i64,
    pub resolved_tickets_today: i64,
    pub avg_first_response_time_minutes: Option<f64>,
    pub avg_resolution_time_hours: Option<f64>,
    pub first_contact_resolution_rate: Option<f64>,
    pub sla_compliance_rate: Option<f64>,
    pub sla_breach_count: i64,
    pub avg_csat_score: Option<f64>,
}

#[derive(Debug, Clone, FromRow, SimpleObject)]
#[graphql(name = "CrmCoreTicketStatusCount")]
pub struct CrmCoreTicketStatusCount {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Clone, FromRow, SimpleObject)]
#[graphql(name = "CrmCoreTicketPriorityCount")]
pub struct CrmCoreTicketPriorityCount {
    pub priority: String,
    pub count: i64,
}

#[derive(Debug, Clone, FromRow, SimpleObject)]
#[graphql(name = "CrmCoreSlaMetrics")]
pub struct CrmCoreSlaMetrics {
    pub total_tickets: i64,
    pub tickets_meeting_sla: i64,
    pub tickets_breaching_sla: i64,
    pub compliance_rate: f64,
    pub avg_first_response_minutes: Option<f64>,
    pub avg_resolution_hours: Option<f64>,
}

#[derive(Debug, Clone, FromRow, SimpleObject)]
#[graphql(name = "CrmCoreResponseMetrics")]
pub struct CrmCoreResponseMetrics {
    pub avg_first_response_minutes: Option<f64>,
    pub median_first_response_minutes: Option<f64>,
    pub avg_response_minutes: Option<f64>,
    pub median_response_minutes: Option<f64>,
    pub avg_resolution_hours: Option<f64>,
    pub median_resolution_hours: Option<f64>,
}

#[derive(Debug, Clone, FromRow, SimpleObject)]
#[graphql(name = "CrmCoreAgentPerformance")]
pub struct CrmCoreAgentPerformance {
    pub agent_id: String,
    pub agent_name: String,
    pub tickets_assigned: i64,
    pub tickets_resolved: i64,
    pub avg_first_response_minutes: Option<f64>,
    pub avg_resolution_hours: Option<f64>,
    pub csat_score: Option<f64>,
}

#[derive(Debug, Clone, FromRow, SimpleObject)]
#[graphql(name = "CrmCoreTicketTrend")]
pub struct CrmCoreTicketTrend {
    pub date: String,
    pub new_tickets: i64,
    pub resolved_tickets: i64,
    pub active_tickets: i64,
}

// Input types
#[derive(Debug, Clone, InputObject)]
pub struct CreateTicketInput {
    pub customer_id: Uuid,
    pub subject: String,
    pub description: String,
    pub priority: TicketPriority,
    pub category: Option<String>,
}

#[derive(Debug, Clone, InputObject)]
pub struct UpdateTicketInput {
    pub subject: Option<String>,
    pub description: Option<String>,
    pub status: Option<TicketStatus>,
    pub priority: Option<TicketPriority>,
    pub category: Option<String>,
    pub assigned_to: Option<Uuid>,
}

#[derive(Debug, Clone, InputObject)]
pub struct AddTicketMessageInput {
    pub ticket_id: Uuid,
    pub content: String,
    pub is_internal: bool,
}

#[derive(Debug, Clone, InputObject)]
pub struct TicketFilter {
    pub status: Option<TicketStatus>,
    pub priority: Option<TicketPriority>,
    pub assigned_to: Option<Uuid>,
    pub customer_id: Option<Uuid>,
    pub category: Option<String>,
    pub search_query: Option<String>,
}
