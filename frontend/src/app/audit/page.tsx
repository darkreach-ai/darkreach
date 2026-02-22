"use client";

/**
 * @module audit/page
 *
 * Admin page for reviewing API audit log entries. Shows a filterable,
 * paginated table of all recorded API actions with user, method, status,
 * and IP information. Useful for security monitoring and compliance.
 */

import { useState, useMemo } from "react";
import { RefreshCw, ChevronLeft, ChevronRight } from "lucide-react";

import { ViewHeader } from "@/components/view-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { relativeTime } from "@/lib/format";
import { useAuditLog } from "@/hooks/use-audit-log";

/** Predefined action types for the filter dropdown. */
const ACTION_OPTIONS = [
  { value: "all", label: "All actions" },
  { value: "search.create", label: "Search Create" },
  { value: "search.stop", label: "Search Stop" },
  { value: "search.pause", label: "Search Pause" },
  { value: "search.resume", label: "Search Resume" },
  { value: "project.create", label: "Project Create" },
  { value: "project.activate", label: "Project Activate" },
  { value: "project.pause", label: "Project Pause" },
  { value: "project.cancel", label: "Project Cancel" },
  { value: "release.upsert", label: "Release Upsert" },
  { value: "release.rollout", label: "Release Rollout" },
  { value: "release.rollback", label: "Release Rollback" },
  { value: "agent.create", label: "Agent Create" },
  { value: "agent.cancel", label: "Agent Cancel" },
  { value: "strategy.override", label: "Strategy Override" },
  { value: "schedule.create", label: "Schedule Create" },
  { value: "schedule.update", label: "Schedule Update" },
  { value: "schedule.delete", label: "Schedule Delete" },
];

const LIMIT = 50;

/**
 * Returns a Tailwind class string for color-coding HTTP status codes.
 * - 2xx: green (success)
 * - 4xx: yellow (client error)
 * - 5xx: red (server error)
 * - Other/unknown: muted
 */
function statusBadgeVariant(
  code: number | null
): "default" | "secondary" | "destructive" | "outline" {
  if (code === null) return "outline";
  if (code >= 200 && code < 300) return "secondary";
  if (code >= 400 && code < 500) return "default";
  if (code >= 500) return "destructive";
  return "outline";
}

function statusBadgeClass(code: number | null): string {
  if (code === null) return "bg-zinc-500/15 text-zinc-400";
  if (code >= 200 && code < 300) return "bg-emerald-500/15 text-emerald-400";
  if (code >= 400 && code < 500) return "bg-amber-500/15 text-amber-400";
  if (code >= 500) return "bg-red-500/15 text-red-400";
  return "bg-zinc-500/15 text-zinc-400";
}

/** Format an HTTP method with color styling. */
function methodClass(method: string): string {
  switch (method.toUpperCase()) {
    case "GET":
      return "text-blue-400";
    case "POST":
      return "text-emerald-400";
    case "PUT":
      return "text-amber-400";
    case "DELETE":
      return "text-red-400";
    case "PATCH":
      return "text-violet-400";
    default:
      return "text-zinc-400";
  }
}

export default function AuditPage() {
  const [page, setPage] = useState(1);
  const [actionFilter, setActionFilter] = useState("all");
  const [userIdFilter, setUserIdFilter] = useState("");

  const { entries, total, isLoading, error, refetch } = useAuditLog({
    page,
    limit: LIMIT,
    action: actionFilter === "all" ? undefined : actionFilter,
    userId: userIdFilter.trim() || undefined,
  });

  const totalPages = Math.max(1, Math.ceil(total / LIMIT));

  const headerMeta = useMemo(() => {
    if (error) return `Error: ${error}`;
    return `${total} entries · page ${page}/${totalPages}`;
  }, [total, page, totalPages, error]);

  function handleActionChange(value: string) {
    setActionFilter(value);
    setPage(1);
  }

  function handleUserIdChange(value: string) {
    setUserIdFilter(value);
    // Reset to page 1 will happen on next render via the hook
  }

  function handleUserIdSubmit() {
    setPage(1);
  }

  return (
    <>
      <ViewHeader
        title="Audit Log"
        subtitle={headerMeta}
        actions={
          <Button
            variant="outline"
            size="sm"
            onClick={() => void refetch()}
            disabled={isLoading}
          >
            <RefreshCw className="size-4" />
            Refresh
          </Button>
        }
      />

      {/* Filters */}
      <Card className="mb-4">
        <CardHeader className="pb-2">
          <CardTitle className="text-xs font-medium text-muted-foreground">
            Filters
          </CardTitle>
        </CardHeader>
        <CardContent className="grid grid-cols-1 md:grid-cols-3 gap-2">
          <Select value={actionFilter} onValueChange={handleActionChange}>
            <SelectTrigger className="h-8 text-xs">
              <SelectValue placeholder="Action" />
            </SelectTrigger>
            <SelectContent>
              {ACTION_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Input
            value={userIdFilter}
            onChange={(e) => handleUserIdChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleUserIdSubmit();
            }}
            placeholder="User ID"
            className="h-8 text-xs"
          />
          <div className="flex items-center text-xs text-muted-foreground">
            {isLoading ? "Loading..." : `${total} matching entries`}
          </div>
        </CardContent>
      </Card>

      {/* Table */}
      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-[140px]">Time</TableHead>
                <TableHead className="w-[180px]">User</TableHead>
                <TableHead className="w-[160px]">Action</TableHead>
                <TableHead>Path</TableHead>
                <TableHead className="w-[70px]">Method</TableHead>
                <TableHead className="w-[70px]">Status</TableHead>
                <TableHead className="w-[120px]">IP</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {entries.length === 0 && (
                <TableRow>
                  <TableCell
                    colSpan={7}
                    className="text-xs text-muted-foreground text-center py-8"
                  >
                    {isLoading
                      ? "Loading audit log..."
                      : error
                        ? `Error: ${error}`
                        : "No audit log entries found."}
                  </TableCell>
                </TableRow>
              )}
              {entries.map((entry) => (
                <TableRow key={entry.id}>
                  <TableCell className="text-xs">
                    <div className="text-muted-foreground">
                      {new Date(entry.created_at).toLocaleString()}
                    </div>
                    <div className="text-[11px] text-muted-foreground/70">
                      {relativeTime(entry.created_at)}
                    </div>
                  </TableCell>
                  <TableCell className="text-xs">
                    <div className="truncate max-w-[180px]" title={entry.user_id}>
                      {entry.user_email || entry.user_id}
                    </div>
                    {entry.user_email && (
                      <div
                        className="text-[11px] text-muted-foreground truncate max-w-[180px]"
                        title={entry.user_id}
                      >
                        {entry.user_id.slice(0, 8)}...
                      </div>
                    )}
                  </TableCell>
                  <TableCell className="text-xs">
                    <Badge variant="outline" className="text-[11px] font-mono">
                      {entry.action}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-xs text-muted-foreground">
                    <div className="truncate max-w-[300px]" title={entry.resource || ""}>
                      {entry.resource || "-"}
                    </div>
                  </TableCell>
                  <TableCell className="text-xs">
                    <span className={`font-mono font-semibold ${methodClass(entry.method)}`}>
                      {entry.method}
                    </span>
                  </TableCell>
                  <TableCell>
                    <Badge
                      variant={statusBadgeVariant(entry.status_code)}
                      className={`text-[11px] font-mono ${statusBadgeClass(entry.status_code)}`}
                    >
                      {entry.status_code ?? "-"}
                    </Badge>
                  </TableCell>
                  <TableCell className="text-xs text-muted-foreground font-mono">
                    {entry.ip_address || "-"}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between mt-4">
          <div className="text-xs text-muted-foreground">
            Showing {(page - 1) * LIMIT + 1}
            {" - "}
            {Math.min(page * LIMIT, total)} of {total}
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              disabled={page <= 1 || isLoading}
            >
              <ChevronLeft className="size-4" />
              Previous
            </Button>
            <span className="text-xs text-muted-foreground px-2">
              Page {page} of {totalPages}
            </span>
            <Button
              variant="outline"
              size="sm"
              onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              disabled={page >= totalPages || isLoading}
            >
              Next
              <ChevronRight className="size-4" />
            </Button>
          </div>
        </div>
      )}
    </>
  );
}
