'use client';

import { useState } from 'react';
import type { Report } from '@/lib/api';

interface ReportViewerProps {
  report: Report;
}

type TabId = 'summary' | 'modules' | 'gating' | 'licensing' | 'inventory';

const TABS: { id: TabId; label: string }[] = [
  { id: 'summary', label: 'Executive Summary' },
  { id: 'modules', label: 'Modules' },
  { id: 'gating', label: 'Gating Recommendations' },
  { id: 'licensing', label: 'Licensing' },
  { id: 'inventory', label: 'Full Inventory' },
];

export default function ReportViewer({ report }: ReportViewerProps) {
  const [activeTab, setActiveTab] = useState<TabId>('summary');

  const modules = report.modules ?? [];
  const tierAssignments = report.gating_strategy?.tier_assignments ?? [];
  const risks = report.risks ?? [];

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <div className="flex gap-4 border-b mb-6 flex-wrap">
        {TABS.map((tab) => (
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
          <section>
            <h2 className="text-xl font-bold mb-4">Executive Summary</h2>
            <ul className="space-y-1 text-slate-700">
              <li>Repository: {report.repository?.name ?? '—'}</li>
              <li>URL: {report.repository?.url ?? '—'}</li>
              <li>Total LOC: {report.repository?.metrics?.total_loc ?? 0}</li>
              <li>Complete: {report.is_complete ? 'yes' : 'no'}</li>
              <li>Risks identified: {risks.length}</li>
            </ul>
            {report.gating_strategy?.boundary_description && (
              <p className="mt-4 text-slate-700">
                {report.gating_strategy.boundary_description}
              </p>
            )}
          </section>
        )}

        {activeTab === 'modules' && (
          <section>
            <h2 className="text-xl font-bold mb-4">Modules</h2>
            {modules.length === 0 ? (
              <p className="text-slate-500">No modules reported.</p>
            ) : (
              <table className="w-full text-left text-sm">
                <thead>
                  <tr className="border-b">
                    <th className="py-2">Name</th>
                    <th>Path</th>
                    <th>Layer</th>
                    <th>LOC</th>
                    <th>Tier</th>
                  </tr>
                </thead>
                <tbody>
                  {modules.map((m) => (
                    <tr key={m.id} className="border-b">
                      <td className="py-2 font-medium">{m.name}</td>
                      <td>{m.path}</td>
                      <td>{m.layer}</td>
                      <td>{m.loc}</td>
                      <td>{m.recommended_tier ?? '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </section>
        )}

        {activeTab === 'gating' && (
          <section>
            <h2 className="text-xl font-bold mb-4">Gating Recommendations</h2>
            {tierAssignments.length === 0 ? (
              <p className="text-slate-500">No tier assignments.</p>
            ) : (
              <ul className="space-y-2">
                {tierAssignments.map((a) => (
                  <li key={a.module_id} className="flex justify-between border-b py-2">
                    <span className="font-medium">{a.module_name}</span>
                    <span className="text-blue-600">{a.tier}</span>
                  </li>
                ))}
              </ul>
            )}
          </section>
        )}

        {activeTab === 'licensing' && (
          <section>
            <h2 className="text-xl font-bold mb-4">Licensing</h2>
            <p className="text-slate-700">
              Primary license: {report.repository?.license ?? 'not detected'}
            </p>
          </section>
        )}

        {activeTab === 'inventory' && (
          <section>
            <h2 className="text-xl font-bold mb-4">Full Inventory</h2>
            {risks.length === 0 ? (
              <p className="text-slate-500">No risks recorded.</p>
            ) : (
              <ul className="space-y-2">
                {risks.map((r, i) => (
                  <li key={i} className="border-b py-2">
                    <span className="font-medium">{r.kind}</span>{' '}
                    <span className="text-slate-500">({r.severity})</span>: {r.description}
                  </li>
                ))}
              </ul>
            )}
          </section>
        )}
      </div>
    </div>
  );
}
