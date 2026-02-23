"use client";

/**
 * @module earnings-history-table
 *
 * Paginated table of individual credit transactions from the operator's
 * credit ledger. Shows block ID, credit amount, reason, and timestamp.
 */

import { useState } from "react";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ChevronLeft, ChevronRight } from "lucide-react";
import type { CreditRow } from "@/hooks/use-earnings";
import { formatCredits, relativeTime } from "@/lib/format";

interface EarningsHistoryTableProps {
  credits: CreditRow[];
  pageSize?: number;
}

const REASON_LABELS: Record<string, { label: string; variant: "default" | "secondary" | "outline" }> = {
  block_completed: { label: "Block", variant: "secondary" },
  prime_discovered: { label: "Discovery", variant: "default" },
};

export function EarningsHistoryTable({
  credits,
  pageSize = 10,
}: EarningsHistoryTableProps) {
  const [page, setPage] = useState(0);
  const totalPages = Math.max(1, Math.ceil(credits.length / pageSize));
  const pageItems = credits.slice(page * pageSize, (page + 1) * pageSize);

  if (credits.length === 0) {
    return (
      <div className="flex items-center justify-center h-24 text-sm text-muted-foreground">
        No credit transactions yet
      </div>
    );
  }

  return (
    <div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-20">Block</TableHead>
            <TableHead className="w-24">Credits</TableHead>
            <TableHead className="w-28">Reason</TableHead>
            <TableHead>Time</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {pageItems.map((row) => {
            const reasonInfo = REASON_LABELS[row.reason ?? ""] ?? {
              label: row.reason ?? "—",
              variant: "outline" as const,
            };
            return (
              <TableRow key={row.id}>
                <TableCell className="font-mono text-xs tabular-nums">
                  {row.block_id != null ? `#${row.block_id}` : "—"}
                </TableCell>
                <TableCell className="font-mono text-xs tabular-nums text-emerald-400">
                  +{formatCredits(row.credit)}
                </TableCell>
                <TableCell>
                  <Badge variant={reasonInfo.variant} className="text-[10px]">
                    {reasonInfo.label}
                  </Badge>
                </TableCell>
                <TableCell className="text-xs text-muted-foreground">
                  {relativeTime(row.granted_at)}
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
      {totalPages > 1 && (
        <div className="flex items-center justify-between px-2 pt-3">
          <span className="text-xs text-muted-foreground">
            Page {page + 1} of {totalPages}
          </span>
          <div className="flex gap-1">
            <Button
              variant="outline"
              size="icon"
              className="h-7 w-7"
              disabled={page === 0}
              onClick={() => setPage(page - 1)}
            >
              <ChevronLeft className="h-3.5 w-3.5" />
            </Button>
            <Button
              variant="outline"
              size="icon"
              className="h-7 w-7"
              disabled={page >= totalPages - 1}
              onClick={() => setPage(page + 1)}
            >
              <ChevronRight className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
