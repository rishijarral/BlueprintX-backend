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

-- ============================================================================
-- Stream A: Blueprint Ingestion & Extraction Tables
-- ============================================================================

-- Processing jobs for document ingestion
CREATE TABLE IF NOT EXISTS processing_jobs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    status VARCHAR(50) DEFAULT 'queued' CHECK (status IN ('queued', 'running', 'paused', 'completed', 'failed', 'cancelled')),
    current_step VARCHAR(100),
    progress DECIMAL(5, 2) DEFAULT 0 CHECK (progress >= 0 AND progress <= 100),
    total_steps INTEGER DEFAULT 0,
    completed_steps INTEGER DEFAULT 0,
    error_message TEXT,
    error_step VARCHAR(100),
    can_retry BOOLEAN DEFAULT TRUE,
    retry_count INTEGER DEFAULT 0,
    max_retries INTEGER DEFAULT 3,
    paused_at TIMESTAMP WITH TIME ZONE,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Processing step details for granular tracking
CREATE TABLE IF NOT EXISTS processing_steps (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    job_id UUID NOT NULL REFERENCES processing_jobs(id) ON DELETE CASCADE,
    step_name VARCHAR(100) NOT NULL,
    step_key VARCHAR(50) NOT NULL,
    step_order INTEGER NOT NULL,
    status VARCHAR(50) DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'completed', 'failed', 'skipped')),
    progress DECIMAL(5, 2) DEFAULT 0 CHECK (progress >= 0 AND progress <= 100),
    message TEXT,
    details JSONB DEFAULT '{}',
    items_total INTEGER DEFAULT 0,
    items_processed INTEGER DEFAULT 0,
    error_message TEXT,
    started_at TIMESTAMP WITH TIME ZONE,
    completed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Extracted materials from blueprints
CREATE TABLE IF NOT EXISTS extracted_materials (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    document_id UUID REFERENCES documents(id) ON DELETE SET NULL,
    job_id UUID REFERENCES processing_jobs(id) ON DELETE SET NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    quantity DECIMAL(15, 4),
    unit VARCHAR(50),
    unit_cost DECIMAL(15, 2),
    total_cost DECIMAL(15, 2),
    location VARCHAR(255),
    room VARCHAR(255),
    specification TEXT,
    trade_category VARCHAR(100),
    csi_division VARCHAR(20),
    source_page INTEGER,
    confidence DECIMAL(3, 2) DEFAULT 0.8 CHECK (confidence >= 0 AND confidence <= 1),
    is_verified BOOLEAN DEFAULT FALSE,
    verified_by UUID REFERENCES profiles(id),
    verified_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Extracted rooms/spaces from blueprints
CREATE TABLE IF NOT EXISTS extracted_rooms (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    document_id UUID REFERENCES documents(id) ON DELETE SET NULL,
    job_id UUID REFERENCES processing_jobs(id) ON DELETE SET NULL,
    room_name VARCHAR(255) NOT NULL,
    room_number VARCHAR(50),
    room_type VARCHAR(100),
    floor VARCHAR(50),
    area_sqft DECIMAL(10, 2),
    ceiling_height DECIMAL(6, 2),
    perimeter_ft DECIMAL(10, 2),
    finishes JSONB DEFAULT '{}',
    fixtures JSONB DEFAULT '[]',
    notes TEXT,
    source_page INTEGER,
    confidence DECIMAL(3, 2) DEFAULT 0.8 CHECK (confidence >= 0 AND confidence <= 1),
    is_verified BOOLEAN DEFAULT FALSE,
    verified_by UUID REFERENCES profiles(id),
    verified_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Project milestones (AI-suggested or manually created)
CREATE TABLE IF NOT EXISTS project_milestones (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    job_id UUID REFERENCES processing_jobs(id) ON DELETE SET NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    phase VARCHAR(100),
    phase_order INTEGER DEFAULT 0,
    estimated_duration_days INTEGER,
    estimated_start_date TIMESTAMP WITH TIME ZONE,
    estimated_end_date TIMESTAMP WITH TIME ZONE,
    actual_start_date TIMESTAMP WITH TIME ZONE,
    actual_end_date TIMESTAMP WITH TIME ZONE,
    dependencies JSONB DEFAULT '[]',
    trades_involved JSONB DEFAULT '[]',
    deliverables JSONB DEFAULT '[]',
    status VARCHAR(50) DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'completed', 'delayed', 'cancelled')),
    progress DECIMAL(5, 2) DEFAULT 0 CHECK (progress >= 0 AND progress <= 100),
    is_ai_generated BOOLEAN DEFAULT TRUE,
    is_verified BOOLEAN DEFAULT FALSE,
    verified_by UUID REFERENCES profiles(id),
    verified_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Extracted trade scopes (structured from AI extraction)
CREATE TABLE IF NOT EXISTS extracted_trade_scopes (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    document_id UUID REFERENCES documents(id) ON DELETE SET NULL,
    job_id UUID REFERENCES processing_jobs(id) ON DELETE SET NULL,
    trade VARCHAR(100) NOT NULL,
    trade_display_name VARCHAR(255),
    csi_division VARCHAR(20),
    inclusions JSONB DEFAULT '[]',
    exclusions JSONB DEFAULT '[]',
    required_sheets JSONB DEFAULT '[]',
    spec_sections JSONB DEFAULT '[]',
    rfi_needed JSONB DEFAULT '[]',
    assumptions JSONB DEFAULT '[]',
    estimated_value DECIMAL(15, 2),
    confidence DECIMAL(3, 2) DEFAULT 0.8 CHECK (confidence >= 0 AND confidence <= 1),
    is_verified BOOLEAN DEFAULT FALSE,
    verified_by UUID REFERENCES profiles(id),
    verified_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- ============================================================================
-- Stream B: Marketplace & Hiring Tables
-- ============================================================================

-- External subcontractors (added by GC, not on platform)
CREATE TABLE IF NOT EXISTS external_subcontractors (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    added_by UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    company_name VARCHAR(255) NOT NULL,
    contact_name VARCHAR(255),
    contact_email VARCHAR(255),
    contact_phone VARCHAR(50),
    trade VARCHAR(100) NOT NULL,
    secondary_trades JSONB DEFAULT '[]',
    location VARCHAR(255),
    address TEXT,
    license_number VARCHAR(100),
    insurance_info TEXT,
    notes TEXT,
    rating DECIMAL(2, 1) DEFAULT 0 CHECK (rating >= 0 AND rating <= 5),
    projects_together INTEGER DEFAULT 0,
    is_preferred BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Hire requests (GC to Subcontractor)
CREATE TABLE IF NOT EXISTS hire_requests (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    tender_id UUID REFERENCES tenders(id) ON DELETE SET NULL,
    gc_id UUID NOT NULL REFERENCES profiles(id),
    subcontractor_id UUID REFERENCES subcontractors(id) ON DELETE SET NULL,
    external_sub_id UUID REFERENCES external_subcontractors(id) ON DELETE SET NULL,
    status VARCHAR(50) DEFAULT 'draft' CHECK (status IN (
        'draft', 'pending', 'sent', 'viewed', 'interested', 'negotiating', 
        'contract_sent', 'contract_signed', 'hired', 'declined', 'cancelled', 'expired'
    )),
    trade VARCHAR(100) NOT NULL,
    title VARCHAR(255) NOT NULL,
    message TEXT,
    scope_description TEXT,
    proposed_amount DECIMAL(15, 2),
    rate_type VARCHAR(50) CHECK (rate_type IN ('fixed', 'hourly', 'daily', 'weekly', 'per_unit', 'negotiable')),
    unit_description VARCHAR(255),
    estimated_hours INTEGER,
    estimated_start_date TIMESTAMP WITH TIME ZONE,
    estimated_end_date TIMESTAMP WITH TIME ZONE,
    response_deadline TIMESTAMP WITH TIME ZONE,
    sub_response TEXT,
    sub_counter_amount DECIMAL(15, 2),
    viewed_at TIMESTAMP WITH TIME ZONE,
    responded_at TIMESTAMP WITH TIME ZONE,
    hired_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    CONSTRAINT hire_request_sub_check CHECK (
        (subcontractor_id IS NOT NULL AND external_sub_id IS NULL) OR
        (subcontractor_id IS NULL AND external_sub_id IS NOT NULL)
    )
);

-- Contract templates
CREATE TABLE IF NOT EXISTS contract_templates (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    template_type VARCHAR(50) NOT NULL CHECK (template_type IN (
        'standard', 'time_materials', 'fixed_price', 'unit_price', 'master_service'
    )),
    content TEXT NOT NULL,
    sections JSONB DEFAULT '[]',
    variables JSONB DEFAULT '[]',
    is_system BOOLEAN DEFAULT FALSE,
    created_by UUID REFERENCES profiles(id),
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Contracts
CREATE TABLE IF NOT EXISTS contracts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    hire_request_id UUID NOT NULL REFERENCES hire_requests(id) ON DELETE CASCADE,
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    template_id UUID REFERENCES contract_templates(id),
    contract_number VARCHAR(50),
    title VARCHAR(255) NOT NULL,
    content TEXT NOT NULL,
    sections JSONB DEFAULT '[]',
    terms_summary TEXT,
    amount DECIMAL(15, 2) NOT NULL,
    payment_schedule JSONB DEFAULT '[]',
    start_date TIMESTAMP WITH TIME ZONE,
    end_date TIMESTAMP WITH TIME ZONE,
    gc_signature TEXT,
    gc_signed_at TIMESTAMP WITH TIME ZONE,
    gc_signed_ip VARCHAR(45),
    sub_signature TEXT,
    sub_signed_at TIMESTAMP WITH TIME ZONE,
    sub_signed_ip VARCHAR(45),
    status VARCHAR(50) DEFAULT 'draft' CHECK (status IN (
        'draft', 'pending_gc', 'pending_sub', 'gc_signed', 'fully_signed', 
        'active', 'completed', 'terminated', 'disputed'
    )),
    pdf_path VARCHAR(500),
    notes TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- In-app messages for hire request negotiation
CREATE TABLE IF NOT EXISTS hire_messages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    hire_request_id UUID NOT NULL REFERENCES hire_requests(id) ON DELETE CASCADE,
    sender_id UUID NOT NULL REFERENCES profiles(id),
    sender_type VARCHAR(10) NOT NULL CHECK (sender_type IN ('gc', 'sub')),
    message TEXT NOT NULL,
    message_type VARCHAR(50) DEFAULT 'text' CHECK (message_type IN (
        'text', 'file', 'counter_offer', 'scope_change', 'schedule_change', 'system'
    )),
    metadata JSONB DEFAULT '{}',
    is_read BOOLEAN DEFAULT FALSE,
    read_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Project team members (hired subcontractors)
CREATE TABLE IF NOT EXISTS project_team (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    hire_request_id UUID REFERENCES hire_requests(id) ON DELETE SET NULL,
    contract_id UUID REFERENCES contracts(id) ON DELETE SET NULL,
    subcontractor_id UUID REFERENCES subcontractors(id) ON DELETE SET NULL,
    external_sub_id UUID REFERENCES external_subcontractors(id) ON DELETE SET NULL,
    role VARCHAR(100),
    trade VARCHAR(100) NOT NULL,
    responsibilities TEXT,
    start_date TIMESTAMP WITH TIME ZONE,
    end_date TIMESTAMP WITH TIME ZONE,
    hourly_rate DECIMAL(10, 2),
    status VARCHAR(50) DEFAULT 'active' CHECK (status IN ('pending', 'active', 'on_hold', 'completed', 'terminated')),
    performance_rating DECIMAL(2, 1) CHECK (performance_rating >= 0 AND performance_rating <= 5),
    notes TEXT,
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    CONSTRAINT project_team_sub_check CHECK (
        (subcontractor_id IS NOT NULL AND external_sub_id IS NULL) OR
        (subcontractor_id IS NULL AND external_sub_id IS NOT NULL) OR
        (subcontractor_id IS NULL AND external_sub_id IS NULL)
    )
);

-- Reviews for subcontractors
CREATE TABLE IF NOT EXISTS subcontractor_reviews (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    subcontractor_id UUID REFERENCES subcontractors(id) ON DELETE CASCADE,
    external_sub_id UUID REFERENCES external_subcontractors(id) ON DELETE CASCADE,
    reviewer_id UUID NOT NULL REFERENCES profiles(id),
    project_id UUID REFERENCES projects(id) ON DELETE SET NULL,
    contract_id UUID REFERENCES contracts(id) ON DELETE SET NULL,
    rating DECIMAL(2, 1) NOT NULL CHECK (rating >= 1 AND rating <= 5),
    quality_rating DECIMAL(2, 1) CHECK (quality_rating >= 1 AND quality_rating <= 5),
    communication_rating DECIMAL(2, 1) CHECK (communication_rating >= 1 AND communication_rating <= 5),
    timeliness_rating DECIMAL(2, 1) CHECK (timeliness_rating >= 1 AND timeliness_rating <= 5),
    value_rating DECIMAL(2, 1) CHECK (value_rating >= 1 AND value_rating <= 5),
    title VARCHAR(255),
    comment TEXT,
    would_hire_again BOOLEAN,
    is_verified BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    CONSTRAINT review_sub_check CHECK (
        (subcontractor_id IS NOT NULL AND external_sub_id IS NULL) OR
        (subcontractor_id IS NULL AND external_sub_id IS NOT NULL)
    )
);

-- ============================================================================
-- Additional Indexes for New Tables
-- ============================================================================

-- Processing jobs indexes
CREATE INDEX IF NOT EXISTS ix_processing_jobs_document_id ON processing_jobs(document_id);
CREATE INDEX IF NOT EXISTS ix_processing_jobs_project_id ON processing_jobs(project_id);
CREATE INDEX IF NOT EXISTS ix_processing_jobs_status ON processing_jobs(status);
CREATE INDEX IF NOT EXISTS ix_processing_steps_job_id ON processing_steps(job_id);

-- Extraction tables indexes
CREATE INDEX IF NOT EXISTS ix_extracted_materials_project_id ON extracted_materials(project_id);
CREATE INDEX IF NOT EXISTS ix_extracted_materials_trade ON extracted_materials(trade_category);
CREATE INDEX IF NOT EXISTS ix_extracted_rooms_project_id ON extracted_rooms(project_id);
CREATE INDEX IF NOT EXISTS ix_project_milestones_project_id ON project_milestones(project_id);
CREATE INDEX IF NOT EXISTS ix_extracted_trade_scopes_project_id ON extracted_trade_scopes(project_id);

-- Hiring tables indexes
CREATE INDEX IF NOT EXISTS ix_external_subcontractors_added_by ON external_subcontractors(added_by);
CREATE INDEX IF NOT EXISTS ix_external_subcontractors_trade ON external_subcontractors(trade);
CREATE INDEX IF NOT EXISTS ix_hire_requests_project_id ON hire_requests(project_id);
CREATE INDEX IF NOT EXISTS ix_hire_requests_gc_id ON hire_requests(gc_id);
CREATE INDEX IF NOT EXISTS ix_hire_requests_subcontractor_id ON hire_requests(subcontractor_id);
CREATE INDEX IF NOT EXISTS ix_hire_requests_status ON hire_requests(status);
CREATE INDEX IF NOT EXISTS ix_contracts_hire_request_id ON contracts(hire_request_id);
CREATE INDEX IF NOT EXISTS ix_contracts_project_id ON contracts(project_id);
CREATE INDEX IF NOT EXISTS ix_contracts_status ON contracts(status);
CREATE INDEX IF NOT EXISTS ix_hire_messages_hire_request_id ON hire_messages(hire_request_id);
CREATE INDEX IF NOT EXISTS ix_hire_messages_sender_id ON hire_messages(sender_id);
CREATE INDEX IF NOT EXISTS ix_project_team_project_id ON project_team(project_id);
CREATE INDEX IF NOT EXISTS ix_subcontractor_reviews_subcontractor_id ON subcontractor_reviews(subcontractor_id);

-- ============================================================================
-- Insert Default Contract Templates
-- ============================================================================

INSERT INTO contract_templates (id, name, description, template_type, content, sections, variables, is_system, is_active)
VALUES 
(
    uuid_generate_v4(),
    'Standard Subcontract Agreement',
    'General terms for any trade - covers scope, payment, timeline, and standard legal clauses',
    'standard',
    E'# SUBCONTRACT AGREEMENT\n\nThis Subcontract Agreement ("Agreement") is entered into as of {{effective_date}} between:\n\n**CONTRACTOR:**\n{{gc_company_name}}\n{{gc_address}}\n\n**SUBCONTRACTOR:**\n{{sub_company_name}}\n{{sub_address}}\n\n## 1. PROJECT\nProject Name: {{project_name}}\nProject Address: {{project_address}}\n\n## 2. SCOPE OF WORK\n{{scope_of_work}}\n\n## 3. CONTRACT PRICE\nThe Contractor agrees to pay the Subcontractor the sum of ${{contract_amount}} for the complete performance of the Work.\n\n## 4. PAYMENT TERMS\n{{payment_terms}}\n\n## 5. SCHEDULE\nStart Date: {{start_date}}\nCompletion Date: {{end_date}}\n\n## 6. INSURANCE REQUIREMENTS\nSubcontractor shall maintain the following minimum insurance coverage:\n- General Liability: $1,000,000 per occurrence\n- Workers Compensation: As required by law\n- Auto Liability: $500,000 combined single limit\n\n## 7. STANDARD TERMS AND CONDITIONS\n{{standard_terms}}\n\n## 8. SIGNATURES\n\n**CONTRACTOR:**\nSignature: _______________________\nName: {{gc_name}}\nTitle: {{gc_title}}\nDate: {{gc_sign_date}}\n\n**SUBCONTRACTOR:**\nSignature: _______________________\nName: {{sub_name}}\nTitle: {{sub_title}}\nDate: {{sub_sign_date}}',
    '[{"key": "parties", "title": "Parties", "editable": true}, {"key": "scope", "title": "Scope of Work", "editable": true}, {"key": "price", "title": "Contract Price", "editable": true}, {"key": "payment", "title": "Payment Terms", "editable": true}, {"key": "schedule", "title": "Schedule", "editable": true}, {"key": "insurance", "title": "Insurance", "editable": false}, {"key": "terms", "title": "Terms & Conditions", "editable": true}]',
    '[{"key": "gc_company_name", "label": "GC Company Name", "type": "text"}, {"key": "gc_address", "label": "GC Address", "type": "text"}, {"key": "sub_company_name", "label": "Sub Company Name", "type": "text"}, {"key": "sub_address", "label": "Sub Address", "type": "text"}, {"key": "project_name", "label": "Project Name", "type": "text"}, {"key": "project_address", "label": "Project Address", "type": "text"}, {"key": "scope_of_work", "label": "Scope of Work", "type": "textarea"}, {"key": "contract_amount", "label": "Contract Amount", "type": "currency"}, {"key": "payment_terms", "label": "Payment Terms", "type": "textarea"}, {"key": "start_date", "label": "Start Date", "type": "date"}, {"key": "end_date", "label": "End Date", "type": "date"}, {"key": "effective_date", "label": "Effective Date", "type": "date"}]',
    TRUE,
    TRUE
),
(
    uuid_generate_v4(),
    'Time & Materials Contract',
    'Hourly/daily rate based contract with material markup',
    'time_materials',
    E'# TIME & MATERIALS SUBCONTRACT\n\nThis Time & Materials Agreement ("Agreement") is entered into as of {{effective_date}} between:\n\n**CONTRACTOR:** {{gc_company_name}}\n**SUBCONTRACTOR:** {{sub_company_name}}\n\n## 1. PROJECT\nProject: {{project_name}} at {{project_address}}\n\n## 2. SCOPE OF WORK\n{{scope_of_work}}\n\n## 3. COMPENSATION\n\n### Labor Rates:\n{{labor_rates}}\n\n### Material Markup: {{material_markup}}%\n\n### Not-to-Exceed Amount: ${{nte_amount}}\n\n## 4. INVOICING\n- Invoices submitted: {{invoice_frequency}}\n- Payment terms: Net {{payment_days}} days\n- Time sheets required: Yes\n- Material receipts required: Yes\n\n## 5. SCHEDULE\nEstimated Duration: {{estimated_duration}}\nStart Date: {{start_date}}\n\n## 6. CHANGE ORDERS\nAny work outside the defined scope requires written authorization.\n\n## 7. SIGNATURES\n\n**CONTRACTOR:** _______________________ Date: ___________\n**SUBCONTRACTOR:** _______________________ Date: ___________',
    '[{"key": "parties", "title": "Parties", "editable": true}, {"key": "scope", "title": "Scope of Work", "editable": true}, {"key": "rates", "title": "Compensation & Rates", "editable": true}, {"key": "invoicing", "title": "Invoicing", "editable": true}, {"key": "schedule", "title": "Schedule", "editable": true}]',
    '[{"key": "labor_rates", "label": "Labor Rates", "type": "textarea"}, {"key": "material_markup", "label": "Material Markup %", "type": "number"}, {"key": "nte_amount", "label": "Not-to-Exceed Amount", "type": "currency"}, {"key": "invoice_frequency", "label": "Invoice Frequency", "type": "select", "options": ["Weekly", "Bi-weekly", "Monthly"]}, {"key": "payment_days", "label": "Payment Days", "type": "number"}, {"key": "estimated_duration", "label": "Estimated Duration", "type": "text"}]',
    TRUE,
    TRUE
),
(
    uuid_generate_v4(),
    'Fixed Price Contract',
    'Lump sum contract with milestone-based payments',
    'fixed_price',
    E'# FIXED PRICE SUBCONTRACT\n\nThis Fixed Price Agreement is entered into as of {{effective_date}}.\n\n**CONTRACTOR:** {{gc_company_name}}\n**SUBCONTRACTOR:** {{sub_company_name}}\n**PROJECT:** {{project_name}}\n\n## 1. FIXED CONTRACT PRICE\nTotal Contract Amount: ${{contract_amount}}\n\n## 2. SCOPE OF WORK\n{{scope_of_work}}\n\n## 3. PAYMENT SCHEDULE\n\n| Milestone | Amount | Due Upon |\n|-----------|--------|----------|\n{{payment_milestones}}\n\n## 4. SCHEDULE\n- Notice to Proceed: {{ntp_date}}\n- Substantial Completion: {{substantial_completion}}\n- Final Completion: {{final_completion}}\n\n## 5. LIQUIDATED DAMAGES\nLiquidated damages of ${{ld_amount}} per day shall apply for delays beyond the completion date.\n\n## 6. RETAINAGE\nRetainage: {{retainage_percent}}% to be released upon final completion.\n\n## 7. INCLUSIONS & EXCLUSIONS\n\n**Included:**\n{{inclusions}}\n\n**Excluded:**\n{{exclusions}}\n\n## 8. SIGNATURES\n\n**CONTRACTOR:** _______________________ Date: ___________\n**SUBCONTRACTOR:** _______________________ Date: ___________',
    '[{"key": "price", "title": "Contract Price", "editable": false}, {"key": "scope", "title": "Scope of Work", "editable": true}, {"key": "milestones", "title": "Payment Milestones", "editable": true}, {"key": "schedule", "title": "Schedule", "editable": true}, {"key": "terms", "title": "Terms", "editable": true}]',
    '[{"key": "payment_milestones", "label": "Payment Milestones", "type": "table"}, {"key": "ntp_date", "label": "Notice to Proceed Date", "type": "date"}, {"key": "substantial_completion", "label": "Substantial Completion", "type": "date"}, {"key": "final_completion", "label": "Final Completion", "type": "date"}, {"key": "ld_amount", "label": "Liquidated Damages/Day", "type": "currency"}, {"key": "retainage_percent", "label": "Retainage %", "type": "number"}, {"key": "inclusions", "label": "Inclusions", "type": "textarea"}, {"key": "exclusions", "label": "Exclusions", "type": "textarea"}]',
    TRUE,
    TRUE
),
(
    uuid_generate_v4(),
    'Unit Price Contract',
    'Per-unit pricing for quantity-based work',
    'unit_price',
    E'# UNIT PRICE SUBCONTRACT\n\nThis Unit Price Agreement is entered into as of {{effective_date}}.\n\n**CONTRACTOR:** {{gc_company_name}}\n**SUBCONTRACTOR:** {{sub_company_name}}\n**PROJECT:** {{project_name}}\n\n## 1. UNIT PRICES\n\n| Item | Description | Unit | Unit Price | Est. Qty | Est. Total |\n|------|-------------|------|------------|----------|------------|\n{{unit_price_schedule}}\n\n**Estimated Contract Value:** ${{estimated_total}}\n\n## 2. SCOPE OF WORK\n{{scope_of_work}}\n\n## 3. MEASUREMENT & PAYMENT\n- Quantities measured: {{measurement_method}}\n- Measurement frequency: {{measurement_frequency}}\n- Payment terms: Net {{payment_days}} days\n\n## 4. QUANTITY VARIATIONS\n- Quantities may vary +/- {{quantity_variance}}% without unit price adjustment\n- Beyond this range, unit prices subject to renegotiation\n\n## 5. SCHEDULE\nStart Date: {{start_date}}\nEstimated Duration: {{estimated_duration}}\n\n## 6. SIGNATURES\n\n**CONTRACTOR:** _______________________ Date: ___________\n**SUBCONTRACTOR:** _______________________ Date: ___________',
    '[{"key": "unit_prices", "title": "Unit Price Schedule", "editable": true}, {"key": "scope", "title": "Scope of Work", "editable": true}, {"key": "measurement", "title": "Measurement & Payment", "editable": true}, {"key": "variations", "title": "Quantity Variations", "editable": true}]',
    '[{"key": "unit_price_schedule", "label": "Unit Price Schedule", "type": "table"}, {"key": "estimated_total", "label": "Estimated Total", "type": "currency"}, {"key": "measurement_method", "label": "Measurement Method", "type": "text"}, {"key": "measurement_frequency", "label": "Measurement Frequency", "type": "select", "options": ["Daily", "Weekly", "Monthly", "Upon Completion"]}, {"key": "quantity_variance", "label": "Quantity Variance %", "type": "number"}]',
    TRUE,
    TRUE
),
(
    uuid_generate_v4(),
    'Master Service Agreement',
    'Framework agreement for ongoing subcontractor relationships',
    'master_service',
    E'# MASTER SERVICE AGREEMENT\n\nThis Master Service Agreement ("MSA") is entered into as of {{effective_date}}.\n\n**CONTRACTOR:** {{gc_company_name}}\n**SUBCONTRACTOR:** {{sub_company_name}}\n\n## 1. PURPOSE\nThis MSA establishes terms for ongoing work assignments ("Work Orders") between the parties.\n\n## 2. TERM\nInitial Term: {{initial_term}}\nRenewal: {{renewal_terms}}\n\n## 3. SERVICES\nSubcontractor shall provide {{trade}} services as detailed in individual Work Orders.\n\n## 4. STANDARD RATES\n\n### Labor:\n{{standard_labor_rates}}\n\n### Materials:\nMarkup: {{material_markup}}%\n\n## 5. WORK ORDER PROCESS\n1. Contractor issues Work Order with scope and budget\n2. Subcontractor accepts or proposes modifications within {{response_days}} days\n3. Work begins upon written acceptance\n\n## 6. INSURANCE REQUIREMENTS\n{{insurance_requirements}}\n\n## 7. PAYMENT TERMS\n- Invoice frequency: {{invoice_frequency}}\n- Payment: Net {{payment_days}} days\n- Retainage: {{retainage_percent}}%\n\n## 8. TERMINATION\nEither party may terminate with {{notice_days}} days written notice.\n\n## 9. GENERAL TERMS\n{{general_terms}}\n\n## 10. SIGNATURES\n\n**CONTRACTOR:** _______________________ Date: ___________\n**SUBCONTRACTOR:** _______________________ Date: ___________',
    '[{"key": "term", "title": "Agreement Term", "editable": true}, {"key": "services", "title": "Services", "editable": true}, {"key": "rates", "title": "Standard Rates", "editable": true}, {"key": "process", "title": "Work Order Process", "editable": true}, {"key": "insurance", "title": "Insurance", "editable": true}, {"key": "payment", "title": "Payment Terms", "editable": true}, {"key": "termination", "title": "Termination", "editable": true}]',
    '[{"key": "initial_term", "label": "Initial Term", "type": "text"}, {"key": "renewal_terms", "label": "Renewal Terms", "type": "text"}, {"key": "trade", "label": "Trade/Service Type", "type": "text"}, {"key": "standard_labor_rates", "label": "Standard Labor Rates", "type": "textarea"}, {"key": "response_days", "label": "Response Days", "type": "number"}, {"key": "insurance_requirements", "label": "Insurance Requirements", "type": "textarea"}, {"key": "notice_days", "label": "Termination Notice Days", "type": "number"}, {"key": "general_terms", "label": "General Terms", "type": "textarea"}]',
    TRUE,
    TRUE
)
ON CONFLICT DO NOTHING;

-- ============================================================================
-- Comments
-- ============================================================================

COMMENT ON TABLE processing_jobs IS 'Tracks document processing/ingestion jobs with real-time progress';
COMMENT ON TABLE processing_steps IS 'Detailed step-by-step progress for processing jobs';
COMMENT ON TABLE extracted_materials IS 'AI-extracted materials from blueprints with quantities';
COMMENT ON TABLE extracted_rooms IS 'AI-extracted room/space information from floor plans';
COMMENT ON TABLE project_milestones IS 'Project milestones - AI-suggested or manually created';
COMMENT ON TABLE extracted_trade_scopes IS 'AI-extracted trade-specific scope breakdown';
COMMENT ON TABLE external_subcontractors IS 'Subcontractors added by GC who are not on the platform';
COMMENT ON TABLE hire_requests IS 'Hire requests from GC to subcontractors';
COMMENT ON TABLE contract_templates IS 'Pre-built contract templates for different agreement types';
COMMENT ON TABLE contracts IS 'Signed contracts between GC and subcontractors';
COMMENT ON TABLE hire_messages IS 'In-app messaging for hire request negotiation';
COMMENT ON TABLE project_team IS 'Team members (hired subcontractors) assigned to projects';
COMMENT ON TABLE subcontractor_reviews IS 'Reviews and ratings for subcontractors';

-- ============================================================================
-- Stream C: Marketplace Enhancement & Notifications
-- ============================================================================

-- Admin flag on profiles (secure admin access)
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS is_admin BOOLEAN DEFAULT FALSE;
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS admin_granted_at TIMESTAMP WITH TIME ZONE;
ALTER TABLE profiles ADD COLUMN IF NOT EXISTS admin_granted_by UUID REFERENCES profiles(id);

-- Notifications table (in-app notifications)
CREATE TABLE IF NOT EXISTS notifications (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    type VARCHAR(50) NOT NULL,
    title VARCHAR(255) NOT NULL,
    message TEXT,
    data JSONB DEFAULT '{}',
    is_read BOOLEAN DEFAULT FALSE,
    read_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Enhance subcontractors table for self-registration and marketplace
DO $$
BEGIN
    -- Verification status
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'verification_status') THEN
        ALTER TABLE subcontractors ADD COLUMN verification_status VARCHAR(20) DEFAULT 'pending';
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'verification_notes') THEN
        ALTER TABLE subcontractors ADD COLUMN verification_notes TEXT;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'verified_at') THEN
        ALTER TABLE subcontractors ADD COLUMN verified_at TIMESTAMP WITH TIME ZONE;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'verified_by') THEN
        ALTER TABLE subcontractors ADD COLUMN verified_by UUID REFERENCES profiles(id);
    END IF;
    
    -- Enhanced profile fields
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'company_description') THEN
        ALTER TABLE subcontractors ADD COLUMN company_description TEXT;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'headline') THEN
        ALTER TABLE subcontractors ADD COLUMN headline VARCHAR(255);
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'year_established') THEN
        ALTER TABLE subcontractors ADD COLUMN year_established INTEGER;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'employee_count') THEN
        ALTER TABLE subcontractors ADD COLUMN employee_count VARCHAR(20);
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'service_areas') THEN
        ALTER TABLE subcontractors ADD COLUMN service_areas JSONB DEFAULT '[]';
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'certifications') THEN
        ALTER TABLE subcontractors ADD COLUMN certifications JSONB DEFAULT '[]';
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'insurance') THEN
        ALTER TABLE subcontractors ADD COLUMN insurance JSONB;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'license_info') THEN
        ALTER TABLE subcontractors ADD COLUMN license_info JSONB;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'min_project_value') THEN
        ALTER TABLE subcontractors ADD COLUMN min_project_value DECIMAL(15,2);
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'max_project_value') THEN
        ALTER TABLE subcontractors ADD COLUMN max_project_value DECIMAL(15,2);
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'availability_status') THEN
        ALTER TABLE subcontractors ADD COLUMN availability_status VARCHAR(20) DEFAULT 'available';
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'response_time_hours') THEN
        ALTER TABLE subcontractors ADD COLUMN response_time_hours INTEGER;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'website') THEN
        ALTER TABLE subcontractors ADD COLUMN website VARCHAR(255);
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractors' AND column_name = 'secondary_trades') THEN
        ALTER TABLE subcontractors ADD COLUMN secondary_trades TEXT[];
    END IF;
END $$;

-- Add constraint for verification_status
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'subcontractors_verification_status_check') THEN
        ALTER TABLE subcontractors ADD CONSTRAINT subcontractors_verification_status_check 
            CHECK (verification_status IN ('pending', 'verified', 'rejected'));
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'subcontractors_availability_status_check') THEN
        ALTER TABLE subcontractors ADD CONSTRAINT subcontractors_availability_status_check 
            CHECK (availability_status IN ('available', 'busy', 'not_taking_work'));
    END IF;
END $$;

-- Enhance tenders for marketplace bidding
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'tenders' AND column_name = 'reserve_price') THEN
        ALTER TABLE tenders ADD COLUMN reserve_price DECIMAL(15,2);
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'tenders' AND column_name = 'visibility') THEN
        ALTER TABLE tenders ADD COLUMN visibility VARCHAR(20) DEFAULT 'public';
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'tenders' AND column_name = 'invited_subcontractors') THEN
        ALTER TABLE tenders ADD COLUMN invited_subcontractors UUID[] DEFAULT '{}';
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'tenders' AND column_name = 'requirements') THEN
        ALTER TABLE tenders ADD COLUMN requirements JSONB DEFAULT '{}';
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'tenders' AND column_name = 'location') THEN
        ALTER TABLE tenders ADD COLUMN location VARCHAR(255);
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'tenders' AND column_name = 'project_name') THEN
        ALTER TABLE tenders ADD COLUMN project_name VARCHAR(255);
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'tenders' AND column_name = 'gc_company_name') THEN
        ALTER TABLE tenders ADD COLUMN gc_company_name VARCHAR(255);
    END IF;
END $$;

-- Add visibility constraint
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'tenders_visibility_check') THEN
        ALTER TABLE tenders ADD CONSTRAINT tenders_visibility_check 
            CHECK (visibility IN ('public', 'invited_only'));
    END IF;
END $$;

-- Enhance bids for competitive marketplace
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'bids' AND column_name = 'breakdown') THEN
        ALTER TABLE bids ADD COLUMN breakdown JSONB DEFAULT '[]';
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'bids' AND column_name = 'proposed_timeline_days') THEN
        ALTER TABLE bids ADD COLUMN proposed_timeline_days INTEGER;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'bids' AND column_name = 'proposed_start_date') THEN
        ALTER TABLE bids ADD COLUMN proposed_start_date DATE;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'bids' AND column_name = 'cover_letter') THEN
        ALTER TABLE bids ADD COLUMN cover_letter TEXT;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'bids' AND column_name = 'attachments') THEN
        ALTER TABLE bids ADD COLUMN attachments JSONB DEFAULT '[]';
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'bids' AND column_name = 'is_winning_bid') THEN
        ALTER TABLE bids ADD COLUMN is_winning_bid BOOLEAN DEFAULT FALSE;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'bids' AND column_name = 'subcontractor_id') THEN
        ALTER TABLE bids ADD COLUMN subcontractor_id UUID REFERENCES subcontractors(id);
    END IF;
END $$;

-- Portfolio projects for subcontractors
CREATE TABLE IF NOT EXISTS portfolio_projects (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    subcontractor_id UUID NOT NULL REFERENCES subcontractors(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    project_type VARCHAR(50),
    trade_category VARCHAR(100),
    location VARCHAR(255),
    completion_date DATE,
    project_value DECIMAL(15,2),
    client_name VARCHAR(255),
    client_testimonial TEXT,
    images JSONB DEFAULT '[]',
    is_featured BOOLEAN DEFAULT FALSE,
    display_order INTEGER DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Saved searches for GCs
CREATE TABLE IF NOT EXISTS saved_searches (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    search_type VARCHAR(20) NOT NULL DEFAULT 'subcontractors',
    filters JSONB NOT NULL,
    notify_new_matches BOOLEAN DEFAULT FALSE,
    last_run_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- Add constraint for search_type
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'saved_searches_type_check') THEN
        ALTER TABLE saved_searches ADD CONSTRAINT saved_searches_type_check 
            CHECK (search_type IN ('subcontractors', 'tenders'));
    END IF;
END $$;

-- Enhance reviews for responses
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractor_reviews' AND column_name = 'response_text') THEN
        ALTER TABLE subcontractor_reviews ADD COLUMN response_text TEXT;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractor_reviews' AND column_name = 'response_at') THEN
        ALTER TABLE subcontractor_reviews ADD COLUMN response_at TIMESTAMP WITH TIME ZONE;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractor_reviews' AND column_name = 'helpful_count') THEN
        ALTER TABLE subcontractor_reviews ADD COLUMN helpful_count INTEGER DEFAULT 0;
    END IF;
    
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'subcontractor_reviews' AND column_name = 'status') THEN
        ALTER TABLE subcontractor_reviews ADD COLUMN status VARCHAR(20) DEFAULT 'published';
    END IF;
END $$;

-- Admin audit log for sensitive operations
CREATE TABLE IF NOT EXISTS admin_audit_log (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    admin_id UUID NOT NULL REFERENCES profiles(id),
    action VARCHAR(100) NOT NULL,
    target_type VARCHAR(50) NOT NULL,
    target_id UUID,
    details JSONB DEFAULT '{}',
    ip_address VARCHAR(45),
    user_agent TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW() NOT NULL
);

-- ============================================================================
-- Marketplace Indexes
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_notifications_user_unread ON notifications(user_id, is_read) WHERE is_read = FALSE;
CREATE INDEX IF NOT EXISTS idx_notifications_user_created ON notifications(user_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_subcontractors_verification ON subcontractors(verification_status);
CREATE INDEX IF NOT EXISTS idx_subcontractors_profile_id ON subcontractors(profile_id);
CREATE INDEX IF NOT EXISTS idx_subcontractors_rating ON subcontractors(rating DESC NULLS LAST);
CREATE INDEX IF NOT EXISTS idx_subcontractors_availability ON subcontractors(availability_status);
CREATE INDEX IF NOT EXISTS idx_tenders_visibility ON tenders(visibility);
CREATE INDEX IF NOT EXISTS idx_tenders_marketplace ON tenders(status, visibility) WHERE status = 'open' AND visibility = 'public';
CREATE INDEX IF NOT EXISTS idx_bids_subcontractor ON bids(subcontractor_id);
CREATE INDEX IF NOT EXISTS idx_bids_winning ON bids(tender_id) WHERE is_winning_bid = TRUE;
CREATE INDEX IF NOT EXISTS idx_portfolio_subcontractor ON portfolio_projects(subcontractor_id);
CREATE INDEX IF NOT EXISTS idx_portfolio_featured ON portfolio_projects(subcontractor_id, is_featured) WHERE is_featured = TRUE;
CREATE INDEX IF NOT EXISTS idx_saved_searches_user ON saved_searches(user_id);
CREATE INDEX IF NOT EXISTS idx_admin_audit_admin ON admin_audit_log(admin_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_admin_audit_target ON admin_audit_log(target_type, target_id);
CREATE INDEX IF NOT EXISTS idx_profiles_admin ON profiles(is_admin) WHERE is_admin = TRUE;

-- Full-text search index for subcontractors
CREATE INDEX IF NOT EXISTS idx_subcontractors_search ON subcontractors 
    USING gin(to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, '') || ' ' || coalesce(trade, '') || ' ' || coalesce(headline, '')));

-- ============================================================================
-- Marketplace Comments
-- ============================================================================

COMMENT ON TABLE notifications IS 'In-app notifications for users';
COMMENT ON TABLE portfolio_projects IS 'Portfolio showcase projects for subcontractors';
COMMENT ON TABLE saved_searches IS 'Saved search filters for quick access';
COMMENT ON TABLE admin_audit_log IS 'Audit trail for admin actions';
COMMENT ON COLUMN profiles.is_admin IS 'Admin flag - grants access to admin panel';
COMMENT ON COLUMN subcontractors.verification_status IS 'Profile verification status: pending, verified, rejected';
COMMENT ON COLUMN tenders.reserve_price IS 'Minimum acceptable bid amount (visible to bidders)';
COMMENT ON COLUMN tenders.visibility IS 'Tender visibility: public (marketplace) or invited_only';
