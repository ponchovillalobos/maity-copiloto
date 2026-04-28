import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Shield, Loader2, AlertTriangle, CheckCircle } from 'lucide-react';
import { toast } from 'sonner';

interface ComplianceReportButtonProps {
  meetingId: string;
}

export function ComplianceReportButton({ meetingId }: ComplianceReportButtonProps) {
  const [loading, setLoading] = useState(false);
  const [complianceStatus, setComplianceStatus] = useState<{
    isLocal: boolean;
    externalCount: number;
  } | null>(null);

  const handleGenerateReport = async () => {
    try {
      setLoading(true);

      // Fetch compliance audit data
      const audit = await invoke<any>('compliance_get_meeting_audit', {
        meetingId,
      });

      // Determine if all endpoints are local
      const isLocalOnly = audit.external_endpoints_detected.length === 0;
      setComplianceStatus({
        isLocal: isLocalOnly,
        externalCount: audit.external_endpoints_detected.length,
      });

      // Export PDF report
      const result = await invoke<any>('compliance_export_report', {
        meetingId,
        outputPath: null,
      });

      const sizeKB = (result.bytes / 1024).toFixed(1);
      toast.success(`Compliance Report guardado: ${sizeKB} KB`, {
        action: {
          label: 'Abrir',
          onClick: () => {
            invoke('show_in_folder', { path: result.path }).catch(console.error);
          },
        },
      });
    } catch (error) {
      console.error('Error generating compliance report:', error);
      toast.error(`No se pudo generar el reporte: ${String(error)}`);
    } finally {
      setLoading(false);
    }
  };

  const getBadgeColor = () => {
    if (!complianceStatus) return 'bg-gray-500/10 text-gray-300';
    return complianceStatus.isLocal
      ? 'bg-green-500/10 text-green-300'
      : 'bg-red-500/10 text-red-300';
  };

  const getBadgeIcon = () => {
    if (!complianceStatus) return null;
    return complianceStatus.isLocal ? (
      <CheckCircle className="w-4 h-4" />
    ) : (
      <AlertTriangle className="w-4 h-4" />
    );
  };

  const getBadgeText = () => {
    if (!complianceStatus) return 'Compliance';
    return complianceStatus.isLocal
      ? '100% local'
      : `${complianceStatus.externalCount} external`;
  };

  return (
    <div className="flex items-center gap-3">
      <button
        onClick={handleGenerateReport}
        disabled={loading}
        className="inline-flex items-center gap-2 px-4 py-2 bg-gradient-to-r from-blue-500/20 to-purple-500/20 hover:from-blue-500/30 hover:to-purple-500/30 border border-blue-500/30 rounded-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed text-sm font-medium text-blue-300"
      >
        {loading ? (
          <Loader2 className="w-4 h-4 animate-spin" />
        ) : (
          <Shield className="w-4 h-4" />
        )}
        {loading ? 'Generando...' : 'Compliance Report'}
      </button>

      {complianceStatus && (
        <div
          className={`inline-flex items-center gap-2 px-3 py-1.5 rounded-full text-xs font-semibold ${getBadgeColor()}`}
        >
          {getBadgeIcon()}
          {getBadgeText()}
        </div>
      )}
    </div>
  );
}
