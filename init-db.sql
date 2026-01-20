-- BlueprintX Database Initialization
-- Creates all required tables for the application

-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ============================================================================
-- Core Tables
-- ============================================================================

-- User profiles (synced with Supabase Auth)
CREATE TABLE IF NOT EXISTS profiles (
    id UUID PRIMARY KEY,
    email VARCHAR(255) NOT NULL UNIQUE,
    user_type VARCHAR(10) NOT NULL DEFAULT 'gc' CHECK (user_type IN ('gc', 'sub')),
    company_name VARCHAR(255),
    first_name VARCHAR(100),
    last_name VARCHAR(100),
    phone VARCHAR(50),
    title VARCHAR(100),
    bio TEXT,
    location VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Projects
CREATE TABLE IF NOT EXISTS projects (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    address VARCHAR(255),
    city VARCHAR(100),
    state VARCHAR(50),
    zip_code VARCHAR(20),
    status VARCHAR(50) DEFAULT 'draft' CHECK (status IN ('draft', 'active', 'bidding', 'awarded', 'in_progress', 'completed', 'cancelled')),
    estimated_value DECIMAL(15, 2),
    bid_due_date TIMESTAMP WITH TIME ZONE,
    start_date TIMESTAMP WITH TIME ZONE,
    end_date TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Documents
CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    document_type VARCHAR(50) DEFAULT 'other' CHECK (document_type IN ('plan', 'specification', 'addendum', 'contract', 'change_order', 'submittal', 'rfi', 'other')),
    file_path VARCHAR(500),
    file_size BIGINT,
    mime_type VARCHAR(100),
    version INTEGER DEFAULT 1,
    status VARCHAR(50) DEFAULT 'active' CHECK (status IN ('draft', 'active', 'superseded', 'archived')),
    category VARCHAR(100),
    revised VARCHAR(50),
    author VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Tenders (bid packages)
CREATE TABLE IF NOT EXISTS tenders (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    trade_category VARCHAR(100) NOT NULL,
    scope_of_work TEXT,
    status VARCHAR(50) DEFAULT 'draft' CHECK (status IN ('draft', 'open', 'closed', 'awarded', 'cancelled')),
    bid_due_date TIMESTAMP WITH TIME ZONE,
    estimated_value DECIMAL(15, 2),
    awarded_to UUID REFERENCES profiles(id),
    priority VARCHAR(20) DEFAULT 'medium' CHECK (priority IN ('low', 'medium', 'high')),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Bids
CREATE TABLE IF NOT EXISTS bids (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    tender_id UUID NOT NULL REFERENCES tenders(id) ON DELETE CASCADE,
    bidder_id UUID REFERENCES profiles(id),
    company_name VARCHAR(255) NOT NULL,
    contact_name VARCHAR(255),
    contact_email VARCHAR(255),
    contact_phone VARCHAR(50),
    bid_amount DECIMAL(15, 2) NOT NULL,
    status VARCHAR(50) DEFAULT 'draft' CHECK (status IN ('draft', 'submitted', 'under_review', 'shortlisted', 'awarded', 'rejected', 'withdrawn')),
    notes TEXT,
    submitted_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Tasks
CREATE TABLE IF NOT EXISTS tasks (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    status VARCHAR(50) DEFAULT 'todo' CHECK (status IN ('todo', 'in_progress', 'completed')),
    priority VARCHAR(20) DEFAULT 'medium' CHECK (priority IN ('low', 'medium', 'high', 'urgent')),
    assignee VARCHAR(255),
    assignee_id UUID REFERENCES profiles(id),
    due_date TIMESTAMP WITH TIME ZONE,
    category VARCHAR(100),
    progress INTEGER DEFAULT 0 CHECK (progress >= 0 AND progress <= 100),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- RFIs (Requests for Information)
CREATE TABLE IF NOT EXISTS rfis (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    number SERIAL,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    status VARCHAR(50) DEFAULT 'open' CHECK (status IN ('open', 'answered', 'closed')),
    priority VARCHAR(20) DEFAULT 'medium' CHECK (priority IN ('low', 'medium', 'high', 'urgent')),
    requester VARCHAR(255) NOT NULL,
    requester_id UUID REFERENCES profiles(id),
    assignee VARCHAR(255) NOT NULL,
    assignee_id UUID REFERENCES profiles(id),
    category VARCHAR(100),
    due_date TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- RFI Responses
CREATE TABLE IF NOT EXISTS rfi_responses (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    rfi_id UUID NOT NULL REFERENCES rfis(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    author VARCHAR(255) NOT NULL,
    author_id UUID NOT NULL REFERENCES profiles(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- User Settings
CREATE TABLE IF NOT EXISTS user_settings (
    user_id UUID PRIMARY KEY REFERENCES profiles(id) ON DELETE CASCADE,
    notification_settings JSONB DEFAULT '{"email_notifications": true, "push_notifications": true, "bid_updates": true, "rfi_alerts": true, "task_reminders": true, "weekly_reports": true}'::jsonb,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Subcontractors (marketplace)
CREATE TABLE IF NOT EXISTS subcontractors (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    profile_id UUID REFERENCES profiles(id),
    name VARCHAR(255) NOT NULL,
    trade VARCHAR(100) NOT NULL,
    rating DECIMAL(2, 1) DEFAULT 0 CHECK (rating >= 0 AND rating <= 5),
    review_count INTEGER DEFAULT 0,
    location VARCHAR(255),
    description TEXT,
    contact_email VARCHAR(255),
    contact_phone VARCHAR(50),
    projects_completed INTEGER DEFAULT 0,
    average_bid_value DECIMAL(15, 2),
    response_time VARCHAR(50),
    verified BOOLEAN DEFAULT FALSE,
    specialties TEXT[],
    recent_projects JSONB DEFAULT '[]'::jsonb,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- ============================================================================
-- Indexes
-- ============================================================================

CREATE INDEX IF NOT EXISTS ix_profiles_email ON profiles(email);
CREATE INDEX IF NOT EXISTS ix_profiles_user_type ON profiles(user_type);

CREATE INDEX IF NOT EXISTS ix_projects_owner_id ON projects(owner_id);
CREATE INDEX IF NOT EXISTS ix_projects_status ON projects(status);

CREATE INDEX IF NOT EXISTS ix_documents_project_id ON documents(project_id);
CREATE INDEX IF NOT EXISTS ix_documents_document_type ON documents(document_type);

CREATE INDEX IF NOT EXISTS ix_tenders_project_id ON tenders(project_id);
CREATE INDEX IF NOT EXISTS ix_tenders_status ON tenders(status);

CREATE INDEX IF NOT EXISTS ix_bids_tender_id ON bids(tender_id);
CREATE INDEX IF NOT EXISTS ix_bids_bidder_id ON bids(bidder_id);

CREATE INDEX IF NOT EXISTS ix_tasks_project_id ON tasks(project_id);
CREATE INDEX IF NOT EXISTS ix_tasks_assignee_id ON tasks(assignee_id);
CREATE INDEX IF NOT EXISTS ix_tasks_status ON tasks(status);

CREATE INDEX IF NOT EXISTS ix_rfis_project_id ON rfis(project_id);
CREATE INDEX IF NOT EXISTS ix_rfis_status ON rfis(status);

CREATE INDEX IF NOT EXISTS ix_rfi_responses_rfi_id ON rfi_responses(rfi_id);

CREATE INDEX IF NOT EXISTS ix_subcontractors_trade ON subcontractors(trade);
CREATE INDEX IF NOT EXISTS ix_subcontractors_location ON subcontractors(location);

-- Document embeddings table for RAG
CREATE TABLE IF NOT EXISTS document_embeddings (
    id VARCHAR(36) PRIMARY KEY,
    content TEXT NOT NULL,
    embedding vector(768) NOT NULL,
    metadata JSONB NOT NULL DEFAULT '{}',
    
    -- Denormalized fields for efficient filtering
    project_id VARCHAR(36),
    document_id VARCHAR(36),
    page_number INTEGER,
    chunk_index INTEGER,
    source VARCHAR(255),
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS ix_document_embeddings_project_id 
    ON document_embeddings(project_id);

CREATE INDEX IF NOT EXISTS ix_document_embeddings_document_id 
    ON document_embeddings(document_id);

CREATE INDEX IF NOT EXISTS ix_document_embeddings_project_document 
    ON document_embeddings(project_id, document_id);

-- IVFFlat index for approximate nearest neighbor search
-- lists = sqrt(n) where n = expected number of rows
-- Start with 100 lists, can be tuned based on data size
CREATE INDEX IF NOT EXISTS ix_document_embeddings_embedding 
    ON document_embeddings 
    USING ivfflat (embedding vector_cosine_ops) 
    WITH (lists = 100);

-- Grant permissions (if using separate app user)
-- GRANT ALL PRIVILEGES ON TABLE document_embeddings TO blueprintx_app;

COMMENT ON TABLE document_embeddings IS 'Vector embeddings for document chunks used in RAG';
COMMENT ON COLUMN document_embeddings.embedding IS 'Gemini gemini-embedding-001 vector (768 dimensions)';
COMMENT ON COLUMN document_embeddings.metadata IS 'Additional metadata as JSON (job_id, start_char, end_char, etc.)';
