'use client';

import { useState } from 'react';
import { useRouter } from 'next/navigation';
import { submitAssessment } from '@/lib/api';

export default function Home() {
  const router = useRouter();
  const [url, setUrl] = useState('');
  const [budget, setBudget] = useState('5');
  const [loading, setLoading] = useState(false);
  const [apiKey, setApiKey] = useState<string>(() => {
    if (typeof window !== 'undefined') {
      return localStorage.getItem('repogate-api-key') ?? '';
    }
    return '';
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    try {
      const data = await submitAssessment(
        { repo_url: url, budget_usd: parseFloat(budget) },
        apiKey,
      );
      localStorage.setItem('repogate-api-key', apiKey);
      router.push(`/jobs?id=${data.job_id}`);
    } catch (error) {
      alert('Error: ' + (error as Error).message);
    } finally {
      setLoading(false);
    }
  };

  return (
    <main className="min-h-screen bg-gradient-to-br from-slate-900 to-slate-800 flex items-center justify-center p-4">
      <div className="bg-white rounded-lg shadow-2xl p-8 max-w-md w-full">
        <div className="flex flex-col items-center mb-4">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img
            src="/repogate.png"
            alt="RepoGate logo"
            width={96}
            height={96}
            className="mb-3"
          />
          <h1 className="text-2xl font-bold text-slate-900">RepoGate</h1>
        </div>
        <p className="text-slate-500 mb-6 text-sm text-center">
          Paste a repository URL to get open-core gating recommendations.
        </p>
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
            placeholder="https://github.com/owner/repo"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            className="w-full px-4 py-2 border rounded-lg"
            required
          />
          <input
            type="number"
            step="0.5"
            min="0"
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
            {loading ? 'Submitting…' : 'Analyze'}
          </button>
        </form>
      </div>
    </main>
  );
}
