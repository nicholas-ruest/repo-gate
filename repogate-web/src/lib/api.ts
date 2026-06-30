// Typed client for the RepoGate HTTP API.
//
// The static export is served from the same origin as the API, so the base is
// empty by default. For local dev against a separate server, set
// NEXT_PUBLIC_API_BASE (e.g. http://localhost:8080).

const API_BASE: string = process.env.NEXT_PUBLIC_API_BASE ?? '';

export interface SubmissionRequest {
  repo_url: string;
  budget_usd: number;
  model_override?: string;
  weights?: Record<string, number>;
}

export interface JobResponse {
  job_id: string;
  estimated_cost_min: number;
  estimated_cost_max: number;
}

export interface JobStatus {
  status: string;
  current_phase: string;
  progress_pct: number;
  tokens_used: number;
}

export interface TierAssignment {
  module_id: string;
  module_name: string;
  tier: string;
  rationale?: string | null;
}

export interface ModuleView {
  id: string;
  name: string;
  path: string;
  layer: string;
  loc: number;
  recommended_tier?: string | null;
}

export interface RiskView {
  kind: string;
  severity: string;
  description: string;
}

export interface Report {
  repo_id?: string;
  schema_version?: string;
  is_complete?: boolean;
  repository?: {
    name?: string;
    url?: string;
    license?: string | null;
    metrics?: { total_files?: number; total_loc?: number };
  };
  modules?: ModuleView[];
  gating_strategy?: { tier_assignments?: TierAssignment[]; boundary_description?: string | null } | null;
  risks?: RiskView[];
}

function authHeaders(apiKey: string): HeadersInit {
  return { Authorization: `Bearer ${apiKey}` };
}

export async function submitAssessment(
  req: SubmissionRequest,
  apiKey: string,
): Promise<JobResponse> {
  const response = await fetch(`${API_BASE}/assessments`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', ...authHeaders(apiKey) },
    body: JSON.stringify(req),
  });
  if (!response.ok) throw new Error(`Submission failed (${response.status})`);
  return (await response.json()) as JobResponse;
}

export async function pollStatus(jobId: string, apiKey: string): Promise<JobStatus> {
  const response = await fetch(`${API_BASE}/assessments/${jobId}/status`, {
    headers: authHeaders(apiKey),
  });
  if (!response.ok) throw new Error(`Poll failed (${response.status})`);
  return (await response.json()) as JobStatus;
}

export async function fetchReport(jobId: string, apiKey: string): Promise<Report> {
  const response = await fetch(`${API_BASE}/assessments/${jobId}`, {
    headers: authHeaders(apiKey),
  });
  if (!response.ok) throw new Error(`Fetch failed (${response.status})`);
  return (await response.json()) as Report;
}
