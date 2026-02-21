-- Migration 011: Customer Support Ticketing System
-- Creates tables for support tickets, messages, and analytics

-- ============================================================================
-- Ticket Status and Priority Enums
-- ============================================================================
CREATE TYPE ticket_status AS ENUM (
    'NEW',
    'IN_PROGRESS',
    'WAITING_ON_CUSTOMER',
    'RESOLVED',
    'CLOSED'
);

CREATE TYPE ticket_priority AS ENUM (
    'LOW',
    'MEDIUM',
    'HIGH',
    'URGENT'
);

-- ============================================================================
-- Support Tickets Table
-- ============================================================================
CREATE TABLE IF NOT EXISTS support_tickets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product VARCHAR(50) NOT NULL,
    customer_id UUID NOT NULL REFERENCES customers(id),
    subject VARCHAR(500) NOT NULL,
    description TEXT NOT NULL,
    status ticket_status NOT NULL DEFAULT 'NEW',
    priority ticket_priority NOT NULL DEFAULT 'MEDIUM',
    category VARCHAR(100),
    assigned_to UUID,  -- References auth.users(id), not enforced by FK
    first_response_at TIMESTAMPTZ,
    resolved_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ,
    sla_breach BOOLEAN NOT NULL DEFAULT FALSE,
    csat_score INTEGER CHECK (csat_score BETWEEN 1 AND 5),
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_support_tickets_product ON support_tickets(product) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_support_tickets_customer_id ON support_tickets(customer_id) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_support_tickets_status ON support_tickets(status) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_support_tickets_priority ON support_tickets(priority) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_support_tickets_assigned_to ON support_tickets(assigned_to) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_support_tickets_created_at ON support_tickets(created_at) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_support_tickets_deleted_at ON support_tickets(deleted_at) WHERE deleted_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_support_tickets_sla_breach ON support_tickets(sla_breach) WHERE deleted_at IS NULL AND sla_breach = TRUE;

-- ============================================================================
-- Ticket Messages Table
-- ============================================================================
CREATE TABLE IF NOT EXISTS ticket_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ticket_id UUID NOT NULL REFERENCES support_tickets(id) ON DELETE CASCADE,
    author_id UUID NOT NULL,  -- References auth.users(id), not enforced by FK
    is_internal BOOLEAN NOT NULL DEFAULT FALSE,
    content TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ticket_messages_ticket_id ON ticket_messages(ticket_id);
CREATE INDEX IF NOT EXISTS idx_ticket_messages_author_id ON ticket_messages(author_id);
CREATE INDEX IF NOT EXISTS idx_ticket_messages_created_at ON ticket_messages(created_at);

-- ============================================================================
-- Trigger: Update updated_at on support_tickets
-- ============================================================================
CREATE OR REPLACE FUNCTION update_support_tickets_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_support_tickets_updated_at
    BEFORE UPDATE ON support_tickets
    FOR EACH ROW
    EXECUTE FUNCTION update_support_tickets_updated_at();

-- ============================================================================
-- Trigger: Auto-set first_response_at on first message
-- ============================================================================
CREATE OR REPLACE FUNCTION set_first_response_at()
RETURNS TRIGGER AS $$
BEGIN
    -- Set first_response_at if this is the first response from an agent
    UPDATE support_tickets
    SET first_response_at = NEW.created_at
    WHERE id = NEW.ticket_id
      AND first_response_at IS NULL
      AND NEW.is_internal = FALSE
      AND NEW.author_id != (SELECT customer_id FROM support_tickets WHERE id = NEW.ticket_id);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_set_first_response_at
    AFTER INSERT ON ticket_messages
    FOR EACH ROW
    EXECUTE FUNCTION set_first_response_at();

-- ============================================================================
-- Trigger: Auto-set resolved_at when status changes to RESOLVED
-- ============================================================================
CREATE OR REPLACE FUNCTION set_resolved_at()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status = 'RESOLVED' AND OLD.status != 'RESOLVED' AND NEW.resolved_at IS NULL THEN
        NEW.resolved_at = NOW();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_set_resolved_at
    BEFORE UPDATE ON support_tickets
    FOR EACH ROW
    EXECUTE FUNCTION set_resolved_at();

-- ============================================================================
-- Trigger: Auto-set closed_at when status changes to CLOSED
-- ============================================================================
CREATE OR REPLACE FUNCTION set_closed_at()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status = 'CLOSED' AND OLD.status != 'CLOSED' AND NEW.closed_at IS NULL THEN
        NEW.closed_at = NOW();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_set_closed_at
    BEFORE UPDATE ON support_tickets
    FOR EACH ROW
    EXECUTE FUNCTION set_closed_at();
