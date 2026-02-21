use chrono::{DateTime, Utc, Duration};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{SupportError, Result};
use crate::models::{
    SupportTicket, TicketMessage, CreateTicketInput, UpdateTicketInput, AddTicketMessageInput,
    TicketFilter, CrmCoreSupportDashboardMetrics, CrmCoreSupportOverviewMetrics, CrmCoreTicketStatusCount,
    CrmCoreTicketPriorityCount, CrmCoreSlaMetrics, CrmCoreResponseMetrics, CrmCoreAgentPerformance, CrmCoreTicketTrend,
};

pub struct SupportRepository {
    pool: PgPool,
}

impl SupportRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new support ticket
    pub async fn create_ticket(&self, product: &str, input: &CreateTicketInput) -> Result<SupportTicket> {
        let ticket = sqlx::query_as::<_, SupportTicket>(
            r#"
            INSERT INTO support_tickets (
                product, customer_id, subject, description, priority, category
            ) VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(product)
        .bind(&input.customer_id)
        .bind(&input.subject)
        .bind(&input.description)
        .bind(&input.priority)
        .bind(&input.category)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create support ticket: {}", e);
            SupportError::Database(e)
        })?;

        Ok(ticket)
    }

    /// Get ticket by ID
    pub async fn find_by_id(&self, ticket_id: Uuid) -> Result<SupportTicket> {
        let ticket = sqlx::query_as::<_, SupportTicket>(
            "SELECT * FROM support_tickets WHERE id = $1 AND deleted_at IS NULL"
        )
        .bind(ticket_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => SupportError::TicketNotFound(ticket_id),
            _ => {
                tracing::error!("Failed to fetch support ticket: {}", e);
                SupportError::Database(e)
            }
        })?;

        Ok(ticket)
    }

    /// Update ticket
    pub async fn update_ticket(&self, ticket_id: Uuid, input: &UpdateTicketInput) -> Result<SupportTicket> {
        let ticket = sqlx::query_as::<_, SupportTicket>(
            r#"
            UPDATE support_tickets SET
                subject = COALESCE($2, subject),
                description = COALESCE($3, description),
                status = COALESCE($4, status),
                priority = COALESCE($5, priority),
                category = COALESCE($6, category),
                assigned_to = COALESCE($7, assigned_to),
                updated_at = NOW()
            WHERE id = $1 AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(ticket_id)
        .bind(&input.subject)
        .bind(&input.description)
        .bind(&input.status)
        .bind(&input.priority)
        .bind(&input.category)
        .bind(&input.assigned_to)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => SupportError::TicketNotFound(ticket_id),
            _ => SupportError::Database(e)
        })?;

        Ok(ticket)
    }

    /// List tickets with filters
    pub async fn list(&self, product: &str, filter: &TicketFilter, limit: i64, offset: i64) -> Result<Vec<SupportTicket>> {
        let mut query = String::from(
            "SELECT * FROM support_tickets WHERE product = $1 AND deleted_at IS NULL"
        );
        let mut params_count = 1;

        if filter.status.is_some() {
            params_count += 1;
            query.push_str(&format!(" AND status = ${}", params_count));
        }

        if filter.priority.is_some() {
            params_count += 1;
            query.push_str(&format!(" AND priority = ${}", params_count));
        }

        if filter.assigned_to.is_some() {
            params_count += 1;
            query.push_str(&format!(" AND assigned_to = ${}", params_count));
        }

        if filter.customer_id.is_some() {
            params_count += 1;
            query.push_str(&format!(" AND customer_id = ${}", params_count));
        }

        query.push_str(" ORDER BY created_at DESC");
        query.push_str(&format!(" LIMIT ${} OFFSET ${}", params_count + 1, params_count + 2));

        let mut q = sqlx::query_as::<_, SupportTicket>(&query)
            .bind(product);

        if let Some(status) = &filter.status {
            q = q.bind(status);
        }
        if let Some(priority) = &filter.priority {
            q = q.bind(priority);
        }
        if let Some(assigned_to) = filter.assigned_to {
            q = q.bind(assigned_to);
        }
        if let Some(customer_id) = filter.customer_id {
            q = q.bind(customer_id);
        }

        q = q.bind(limit).bind(offset);

        let tickets = q.fetch_all(&self.pool)
            .await
            .map_err(|e| SupportError::Database(e))?;

        Ok(tickets)
    }

    /// Add message to ticket
    pub async fn add_message(&self, author_id: Uuid, input: &AddTicketMessageInput) -> Result<TicketMessage> {
        let message = sqlx::query_as::<_, TicketMessage>(
            r#"
            INSERT INTO ticket_messages (ticket_id, author_id, is_internal, content)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(&input.ticket_id)
        .bind(author_id)
        .bind(input.is_internal)
        .bind(&input.content)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| SupportError::Database(e))?;

        Ok(message)
    }

    /// Get messages for a ticket
    pub async fn get_messages(&self, ticket_id: Uuid) -> Result<Vec<TicketMessage>> {
        let messages = sqlx::query_as::<_, TicketMessage>(
            "SELECT * FROM ticket_messages WHERE ticket_id = $1 ORDER BY created_at ASC"
        )
        .bind(ticket_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| SupportError::Database(e))?;

        Ok(messages)
    }

    /// Get dashboard metrics for support analytics
    pub async fn get_dashboard_metrics(
        &self,
        product: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<CrmCoreSupportDashboardMetrics> {
        // Overview metrics
        let overview = self.get_overview_metrics(product, period_start, period_end).await?;

        // Ticket counts by status
        let ticket_by_status = self.get_status_counts(product, period_start, period_end).await?;

        // Ticket counts by priority
        let ticket_by_priority = self.get_priority_counts(product, period_start, period_end).await?;

        // SLA metrics
        let sla_metrics = self.get_sla_metrics(product, period_start, period_end).await?;

        // Response metrics
        let response_metrics = self.get_response_metrics(product, period_start, period_end).await?;

        // Top performing agents
        let top_agents = self.get_top_agents(product, period_start, period_end).await?;

        // Ticket trends (last 7 days)
        let ticket_trends = self.get_ticket_trends(product, period_start, period_end).await?;

        Ok(CrmCoreSupportDashboardMetrics {
            overview,
            ticket_by_status,
            ticket_by_priority,
            sla_metrics,
            response_metrics,
            top_agents,
            ticket_trends,
        })
    }

    async fn get_overview_metrics(
        &self,
        product: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<CrmCoreSupportOverviewMetrics> {
        let metrics = sqlx::query_as::<_, CrmCoreSupportOverviewMetrics>(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE status NOT IN ('CLOSED', 'RESOLVED')) as total_active_tickets,
                COUNT(*) FILTER (WHERE created_at >= NOW() - INTERVAL '1 day') as new_tickets_today,
                COUNT(*) FILTER (WHERE resolved_at >= NOW() - INTERVAL '1 day') as resolved_tickets_today,
                AVG(EXTRACT(EPOCH FROM (first_response_at - created_at)) / 60) FILTER (WHERE first_response_at IS NOT NULL) as avg_first_response_time_minutes,
                AVG(EXTRACT(EPOCH FROM (resolved_at - created_at)) / 3600) FILTER (WHERE resolved_at IS NOT NULL) as avg_resolution_time_hours,
                (COUNT(*) FILTER (WHERE resolved_at IS NOT NULL AND first_response_at IS NOT NULL
                    AND resolved_at - first_response_at < INTERVAL '1 hour')::FLOAT /
                NULLIF(COUNT(*) FILTER (WHERE resolved_at IS NOT NULL), 0)::FLOAT * 100) as first_contact_resolution_rate,
                (COUNT(*) FILTER (WHERE sla_breach = FALSE)::FLOAT /
                NULLIF(COUNT(*), 0)::FLOAT * 100) as sla_compliance_rate,
                COUNT(*) FILTER (WHERE sla_breach = TRUE) as sla_breach_count,
                AVG(csat_score::FLOAT) FILTER (WHERE csat_score IS NOT NULL) as avg_csat_score
            FROM support_tickets
            WHERE product = $1
              AND deleted_at IS NULL
              AND created_at BETWEEN $2 AND $3
            "#,
        )
        .bind(product)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| SupportError::Database(e))?;

        Ok(metrics)
    }

    async fn get_status_counts(
        &self,
        product: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<Vec<CrmCoreTicketStatusCount>> {
        let counts = sqlx::query_as::<_, CrmCoreTicketStatusCount>(
            r#"
            SELECT
                status::TEXT as status,
                COUNT(*)::BIGINT as count
            FROM support_tickets
            WHERE product = $1
              AND deleted_at IS NULL
              AND created_at BETWEEN $2 AND $3
            GROUP BY status
            ORDER BY count DESC
            "#,
        )
        .bind(product)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| SupportError::Database(e))?;

        Ok(counts)
    }

    async fn get_priority_counts(
        &self,
        product: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<Vec<CrmCoreTicketPriorityCount>> {
        let counts = sqlx::query_as::<_, CrmCoreTicketPriorityCount>(
            r#"
            SELECT
                priority::TEXT as priority,
                COUNT(*)::BIGINT as count
            FROM support_tickets
            WHERE product = $1
              AND deleted_at IS NULL
              AND created_at BETWEEN $2 AND $3
            GROUP BY priority
            ORDER BY
                CASE priority::TEXT
                    WHEN 'URGENT' THEN 1
                    WHEN 'HIGH' THEN 2
                    WHEN 'MEDIUM' THEN 3
                    WHEN 'LOW' THEN 4
                END
            "#,
        )
        .bind(product)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| SupportError::Database(e))?;

        Ok(counts)
    }

    async fn get_sla_metrics(
        &self,
        product: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<CrmCoreSlaMetrics> {
        let metrics = sqlx::query_as::<_, CrmCoreSlaMetrics>(
            r#"
            SELECT
                COUNT(*)::BIGINT as total_tickets,
                COUNT(*) FILTER (WHERE sla_breach = FALSE)::BIGINT as tickets_meeting_sla,
                COUNT(*) FILTER (WHERE sla_breach = TRUE)::BIGINT as tickets_breaching_sla,
                COALESCE(
                    COUNT(*) FILTER (WHERE sla_breach = FALSE)::FLOAT /
                    NULLIF(COUNT(*), 0)::FLOAT * 100,
                    0.0
                ) as compliance_rate,
                AVG(EXTRACT(EPOCH FROM (first_response_at - created_at)) / 60) FILTER (WHERE first_response_at IS NOT NULL) as avg_first_response_minutes,
                AVG(EXTRACT(EPOCH FROM (resolved_at - created_at)) / 3600) FILTER (WHERE resolved_at IS NOT NULL) as avg_resolution_hours
            FROM support_tickets
            WHERE product = $1
              AND deleted_at IS NULL
              AND created_at BETWEEN $2 AND $3
            "#,
        )
        .bind(product)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| SupportError::Database(e))?;

        Ok(metrics)
    }

    async fn get_response_metrics(
        &self,
        product: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<CrmCoreResponseMetrics> {
        let metrics = sqlx::query_as::<_, CrmCoreResponseMetrics>(
            r#"
            SELECT
                AVG(EXTRACT(EPOCH FROM (first_response_at - created_at)) / 60) as avg_first_response_minutes,
                PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY EXTRACT(EPOCH FROM (first_response_at - created_at)) / 60) as median_first_response_minutes,
                AVG(EXTRACT(EPOCH FROM (first_response_at - created_at)) / 60) as avg_response_minutes,
                PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY EXTRACT(EPOCH FROM (first_response_at - created_at)) / 60) as median_response_minutes,
                AVG(EXTRACT(EPOCH FROM (resolved_at - created_at)) / 3600) as avg_resolution_hours,
                PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY EXTRACT(EPOCH FROM (resolved_at - created_at)) / 3600) as median_resolution_hours
            FROM support_tickets
            WHERE product = $1
              AND deleted_at IS NULL
              AND created_at BETWEEN $2 AND $3
            "#,
        )
        .bind(product)
        .bind(period_start)
        .bind(period_end)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| SupportError::Database(e))?;

        Ok(metrics)
    }

    async fn get_top_agents(
        &self,
        product: &str,
        period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<Vec<CrmCoreAgentPerformance>> {
        let agents = sqlx::query_as::<_, CrmCoreAgentPerformance>(
            r#"
            SELECT
                assigned_to::TEXT as agent_id,
                assigned_to::TEXT as agent_name,
                COUNT(*)::BIGINT as tickets_assigned,
                COUNT(*) FILTER (WHERE status = 'RESOLVED' OR status = 'CLOSED')::BIGINT as tickets_resolved,
                AVG(EXTRACT(EPOCH FROM (first_response_at - created_at)) / 60) FILTER (WHERE first_response_at IS NOT NULL) as avg_first_response_minutes,
                AVG(EXTRACT(EPOCH FROM (resolved_at - created_at)) / 3600) FILTER (WHERE resolved_at IS NOT NULL) as avg_resolution_hours,
                AVG(csat_score::FLOAT) FILTER (WHERE csat_score IS NOT NULL) as csat_score
            FROM support_tickets
            WHERE product = $1
              AND deleted_at IS NULL
              AND assigned_to IS NOT NULL
              AND created_at BETWEEN $2 AND $3
            GROUP BY assigned_to
            ORDER BY tickets_resolved DESC
            LIMIT 10
            "#,
        )
        .bind(product)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| SupportError::Database(e))?;

        Ok(agents)
    }

    async fn get_ticket_trends(
        &self,
        product: &str,
        _period_start: DateTime<Utc>,
        period_end: DateTime<Utc>,
    ) -> Result<Vec<CrmCoreTicketTrend>> {
        // Get last 7 days of trends
        let start = period_end - Duration::days(7);

        let trends = sqlx::query_as::<_, CrmCoreTicketTrend>(
            r#"
            WITH date_series AS (
                SELECT generate_series($2::DATE, $3::DATE, '1 day'::INTERVAL)::DATE as date
            )
            SELECT
                ds.date::TEXT as date,
                COALESCE(COUNT(*) FILTER (WHERE DATE(created_at) = ds.date), 0)::BIGINT as new_tickets,
                COALESCE(COUNT(*) FILTER (WHERE DATE(resolved_at) = ds.date), 0)::BIGINT as resolved_tickets,
                COALESCE(COUNT(*) FILTER (WHERE status NOT IN ('CLOSED', 'RESOLVED') AND DATE(created_at) <= ds.date), 0)::BIGINT as active_tickets
            FROM date_series ds
            LEFT JOIN support_tickets st ON st.product = $1 AND st.deleted_at IS NULL
            GROUP BY ds.date
            ORDER BY ds.date DESC
            "#,
        )
        .bind(product)
        .bind(start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| SupportError::Database(e))?;

        Ok(trends)
    }
}
