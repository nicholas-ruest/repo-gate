'use client';

import { Suspense, useEffect, useState } from 'react';
import { useSearchParams } from 'next/navigation';
import ReportViewer from '@/components/ReportViewer';
import { fetchReport, pollStatus, type JobStatus, type Report } from '@/lib/api';

function JobView() {
  const searchParams = useSearchParams();
  const jobId = searchParams.get('id') ?? '';
  const [status, setStatus] = useState<JobStatus | null>(null);
  const [report, setReport] = useState<Report | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    if (!jobId) {
      setLoading(false);
      return;
    }
    const apiKey =
      typeof window !== 'undefined' ? localStorage.getItem('repogate-api-key') ?? '' : '';

    let cancelled = false;
    const poll = async () => {
      try {
        const data = await pollStatus(jobId, apiKey);
        if (cancelled) return;
        setStatus(data);
        if (data.status === 'complete') {
          const reportData = await fetchReport(jobId, apiKey);
          if (!cancelled) setReport(reportData);
        }
      } catch (error) {
        console.error('Poll error:', error);
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    poll();
    const interval = setInterval(poll, 3000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [jobId]);

  return (
    <main className="min-h-screen bg-slate-50 p-8">
      <h1 className="text-3xl font-bold mb-6">Assessment {jobId}</h1>

      {!jobId ? (
        <p className="text-red-600">No job id provided.</p>
      ) : loading && !status ? (
        <div>Loading…</div>
      ) : report ? (
        <ReportViewer report={report} />
      ) : (
        <div className="space-y-2">
          <p>Status: {status?.status ?? 'unknown'}</p>
          <p>Phase: {status?.current_phase ?? '—'}</p>
          <progress value={status?.progress_pct ?? 0} max={100} className="w-full" />
        </div>
      )}
    </main>
  );
}

export default function JobsPage() {
  return (
    <Suspense fallback={<div className="p-8">Loading…</div>}>
      <JobView />
    </Suspense>
  );
}
