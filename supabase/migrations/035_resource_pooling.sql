-- Heterogeneous resource pooling — Phase 1: operator_nodes capability columns.
--
-- Extends operator_nodes with GPU runtime, storage role, network role,
-- and a JSONB catch-all for forward compatibility. All columns are optional
-- with defaults that preserve existing CPU-only behavior.

-- GPU capabilities (beyond the existing has_gpu boolean)
ALTER TABLE operator_nodes
  ADD COLUMN IF NOT EXISTS gpu_runtime TEXT DEFAULT 'none',
  ADD COLUMN IF NOT EXISTS gpu_compute_units INTEGER,
  ADD COLUMN IF NOT EXISTS gpu_benchmark_gflops REAL;

-- Storage capabilities
ALTER TABLE operator_nodes
  ADD COLUMN IF NOT EXISTS storage_role TEXT DEFAULT 'none',
  ADD COLUMN IF NOT EXISTS storage_dedicated_gb INTEGER,
  ADD COLUMN IF NOT EXISTS storage_used_gb INTEGER,
  ADD COLUMN IF NOT EXISTS storage_medium TEXT;

-- Network capabilities
ALTER TABLE operator_nodes
  ADD COLUMN IF NOT EXISTS network_role TEXT DEFAULT 'worker',
  ADD COLUMN IF NOT EXISTS network_upload_mbps REAL,
  ADD COLUMN IF NOT EXISTS network_download_mbps REAL,
  ADD COLUMN IF NOT EXISTS network_region TEXT,
  ADD COLUMN IF NOT EXISTS network_public_ip BOOLEAN DEFAULT FALSE;

-- Forward-compatible JSONB catch-all for resource capabilities
ALTER TABLE operator_nodes
  ADD COLUMN IF NOT EXISTS resource_capabilities JSONB DEFAULT '{}';

-- Partial indexes for efficient resource-type queries
CREATE INDEX IF NOT EXISTS idx_operator_nodes_gpu_runtime
  ON operator_nodes (gpu_runtime)
  WHERE gpu_runtime IS NOT NULL AND gpu_runtime <> 'none';

CREATE INDEX IF NOT EXISTS idx_operator_nodes_storage_role
  ON operator_nodes (storage_role)
  WHERE storage_role IS NOT NULL AND storage_role <> 'none';

CREATE INDEX IF NOT EXISTS idx_operator_nodes_network_role
  ON operator_nodes (network_role)
  WHERE network_role IS NOT NULL AND network_role <> 'worker';

CREATE INDEX IF NOT EXISTS idx_operator_nodes_region
  ON operator_nodes (network_region)
  WHERE network_region IS NOT NULL;
