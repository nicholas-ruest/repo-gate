# P16 — `repogate-web`: Next.js Dashboard (TypeScript)

## Context

**You are implementing the web dashboard: form submission, job polling, report viewing.**

**Prerequisites:** P15 (server API) is complete.

---

## Phase & Dependencies

- **Phase:** UX
- **Depends on:** P15

---

## Scope & Deliverables

Implement `repogate-web/` as a Next.js 14+ static export.

### File: `next.config.js`

```javascript
/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  trailingSlash: true,
  rewrites: async () => [
    {
      source: '/api/:path*',
      destination: 'http://localhost:8080/api/:path*',
    },
  ],
};

module.exports = nextConfig;
```

### File: `src/app/page.tsx`

```typescript
'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';

export default function Home() {
  const router = useRouter();
  const [url, setUrl] = useState('');
  const [budget, setBudget] = useState('5');
  const [loading, setLoading] = useState(false);
  const [apiKey, setApiKey] = useState(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem('repogate-api-key') || '';
    }
    return '';
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    
    try {
      const response = await fetch('/api/assessments', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': `Bearer ${apiKey}`,
        },
        body: JSON.stringify({
          repo_url: url,
          budget_usd: parseFloat(budget),
        }),
      });
      
      if (!response.ok) throw new Error('Failed to submit');
      
      const data = await response.json();
      localStorage.setItem('repogate-api-key', apiKey);
      router.push(`/jobs/${data.job_id}`);
    } catch (error) {
      alert('Error: ' + (error as Error).message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <main className="min-h-screen bg-gradient-to-br from-slate-900 to-slate-800 flex items-center justify-center p-4">
      <div className="bg-white rounded-lg shadow-2xl p-8 max-w-md w-full">
        <h1 className="text-2xl font-bold text-slate-900 mb-6">RepoGate</h1>
        <form onSubmit={handleSubmit} className="space-y-4">
          <input
            type="password"
            placeholder="API Key"
            value={apiKey}
            onChange={(e) => setApiKey(e.target.value)}
            className="w-full px-4 py-2 border rounded-lg"
            required
          />
          <input
            type="url"
            placeholder="Repository URL"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            className="w-full px-4 py-2 border rounded-lg"
            required
          />
          <input
            type="number"
            placeholder="Budget (USD)"
            value={budget}
            onChange={(e) => setBudget(e.target.value)}
            className="w-full px-4 py-2 border rounded-lg"
            required
          />
          <button
            type="submit"
            disabled={loading}
            className="w-full bg-blue-600 text-white py-2 rounded-lg hover:bg-blue-700 disabled:opacity-50"
          >
            {loading ? 'Submitting...' : 'Analyze'}
          </button>
        </form>
      </div>
    </main>
  );
}
```

### File: `src/app/jobs/[id]/page.tsx`

```typescript
'use client';

import { useEffect, useState } from 'react';
import { useParams } from 'next/navigation';
import ReportViewer from '@/components/ReportViewer';

export default function JobPage() {
  const params = useParams();
  const jobId = params.id as string;
  const [status, setStatus] = useState<any>(null);
  const [report, setReport] = useState<any>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const poll = async () => {
      try {
        const res = await fetch(`/api/assessments/${jobId}/status`);
        const data = await res.json();
        setStatus(data);
        
        if (data.status === 'complete') {
          const reportRes = await fetch(`/api/assessments/${jobId}/report`);
          const reportData = await reportRes.json();
          setReport(reportData);
        }
      } catch (error) {
        console.error('Poll error:', error);
      } finally {
        setLoading(false);
      }
    };

    const interval = setInterval(poll, 3000);
    poll();
    
    return () => clearInterval(interval);
  }, [jobId]);

  return (
    <main className="min-h-screen bg-slate-50 p-8">
      <h1 className="text-3xl font-bold mb-6">Assessment Status</h1>
      
      {loading ? (
        <div>Loading...</div>
      ) : status?.status === 'complete' && report ? (
        <ReportViewer report={report} />
      ) : (
        <div>
          <p>Status: {status?.status}</p>
          <p>Phase: {status?.current_phase}</p>
          <progress value={status?.progress_pct} max={100} />
        </div>
      )}
    </main>
  );
}
```

### File: `src/components/ReportViewer.tsx`

```typescript
'use client';

import { useState } from 'react';

interface ReportViewerProps {
  report: any;
}

export default function ReportViewer({ report }: ReportViewerProps) {
  const [activeTab, setActiveTab] = useState('summary');

  const tabs = [
    { id: 'summary', label: 'Executive Summary' },
    { id: 'modules', label: 'Modules' },
    { id: 'gating', label: 'Gating Recommendations' },
    { id: 'licensing', label: 'Licensing' },
    { id: 'inventory', label: 'Full Inventory' },
  ];

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <div className="flex gap-4 border-b mb-6">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-2 font-medium ${
              activeTab === tab.id
                ? 'border-b-2 border-blue-600 text-blue-600'
                : 'text-slate-600 hover:text-slate-900'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <div>
        {activeTab === 'summary' && (
          <div>
            <h2 className="text-xl font-bold mb-4">Executive Summary</h2>
            {/* Render summary content */}
          </div>
        )}
        {activeTab === 'modules' && (
          <div>
            <h2 className="text-xl font-bold mb-4">Modules</h2>
            {/* Render module list */}
          </div>
        )}
        {/* Other tabs similarly */}
      </div>
    </div>
  );
}
```

### File: `src/lib/api.ts`

```typescript
export interface SubmissionRequest {
  repo_url: string;
  budget_usd: number;
  model_override?: string;
  weights?: Record<string, number>;
}

export interface JobStatus {
  status: string;
  current_phase: string;
  progress_pct: number;
  tokens_used: number;
}

export async function submitAssessment(req: SubmissionRequest, apiKey: string) {
  const response = await fetch('/api/assessments', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiKey}`,
    },
    body: JSON.stringify(req),
  });
  
  if (!response.ok) throw new Error('Submission failed');
  return response.json();
}

export async function pollStatus(jobId: string, apiKey: string): Promise<JobStatus> {
  const response = await fetch(`/api/assessments/${jobId}/status`, {
    headers: { 'Authorization': `Bearer ${apiKey}` },
  });
  
  if (!response.ok) throw new Error('Poll failed');
  return response.json();
}

export async function fetchReport(jobId: string, apiKey: string) {
  const response = await fetch(`/api/assessments/${jobId}/report`, {
    headers: { 'Authorization': `Bearer ${apiKey}` },
  });
  
  if (!response.ok) throw new Error('Fetch failed');
  return response.json();
}
```

### File: `package.json`

```json
{
  "name": "repogate-web",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "next dev",
    "build": "next build",
    "start": "next start",
    "lint": "next lint"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "next": "^14.0.0"
  },
  "devDependencies": {
    "@types/node": "^20.0.0",
    "@types/react": "^18.0.0",
    "@types/react-dom": "^18.0.0",
    "typescript": "^5.0.0",
    "tailwindcss": "^3.0.0",
    "postcss": "^8.0.0",
    "autoprefixer": "^10.0.0"
  }
}
```

---

## Source Documents to Read

- **`docs/adr/ADR-015-web-api-layer-axum-nextjs.md`** — Static export, 3s polling, 5 tabs, dev proxy
- **`docs/adr/ADR-004-rust-native-orchestration-typescript-scope.md`** — TypeScript scope (web only)

---

## Acceptance Criteria

- ✅ `npm run build` → no TypeScript errors
- ✅ `npm run dev` → form submission calls `POST /assessments`
- ✅ With running server + completed assessment, viewer renders all 5 tabs
- ✅ `next export` → static `out/`; server serves with `--static-dir out/`
- ✅ No console errors

---

## Language

**TypeScript** — React, Next.js, client-side API integration.

---

## Out-of-Scope

- Do NOT implement real-time updates (polling is MVP)
- Do NOT implement user authentication beyond API key
