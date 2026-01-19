-- BlueprintX Database Initialization
-- Enables pgvector extension and creates document embeddings table

-- Enable pgvector extension
CREATE EXTENSION IF NOT EXISTS vector;

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
