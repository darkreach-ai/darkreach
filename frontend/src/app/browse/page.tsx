"use client";

/**
 * @module browse/page
 *
 * Full-width sortable prime table with infinite scroll. Features:
 *
 * - **Sortable columns**: click headers to sort by expression, form, digits, date
 * - **Infinite scroll**: IntersectionObserver pre-fetches 400px before sentinel
 * - **Sticky filter bar**: search, form, digit range — all URL-synced
 * - **Active filter pills**: dismissable badges showing current filters
 * - **Verification status**: green checkmark for verified, gray circle for pending
 * - **Detail dialog**: click a row to see full prime details + verify
 * - **Loading states**: skeleton rows, end-of-results, empty state
 */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import Link from "next/link";
import {
  ArrowDown,
  ArrowUp,
  ArrowUpDown,
  CheckCircle2,
  Circle,
  Download,
  Search,
  SearchX,
  X,
} from "lucide-react";
import { usePrimes, type PrimeFilter, type PrimeRecord } from "@/hooks/use-primes";
import { useStats } from "@/hooks/use-stats";
import {
  API_BASE,
  formLabels,
  formToSlug,
  formatTime,
  numberWithCommas,
  relativeTime,
} from "@/lib/format";
import { cn } from "@/lib/utils";
import { ViewHeader } from "@/components/view-header";
import { PrimeDetailDialog } from "@/components/prime-detail-dialog";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface SortState {
  column: string;
  dir: "asc" | "desc";
}

function parsePositiveInteger(value: string): number | null {
  const trimmed = value.trim();
  if (!trimmed) return 0;
  const parsed = Number(trimmed);
  if (!Number.isInteger(parsed) || parsed < 1) return null;
  return parsed;
}

function SortIcon({ column, sort }: { column: string; sort: SortState }) {
  if (sort.column !== column) {
    return <ArrowUpDown className="size-3.5 text-muted-foreground/40" />;
  }
  return sort.dir === "asc" ? (
    <ArrowUp className="size-3.5 text-foreground" />
  ) : (
    <ArrowDown className="size-3.5 text-foreground" />
  );
}

function SkeletonRow() {
  return (
    <TableRow className="hover:bg-transparent">
      <TableCell className="w-10"><Skeleton className="size-4 mx-auto rounded-full" /></TableCell>
      <TableCell><Skeleton className="h-4 w-52" /></TableCell>
      <TableCell><Skeleton className="h-5 w-20 rounded-full" /></TableCell>
      <TableCell className="text-right"><Skeleton className="h-4 w-16 ml-auto" /></TableCell>
      <TableCell className="text-right"><Skeleton className="h-4 w-20 ml-auto" /></TableCell>
    </TableRow>
  );
}

export default function BrowsePage() {
  const { stats } = useStats();
  const {
    primes,
    selectedPrime,
    fetchPrimeDetail,
    clearSelectedPrime,
    resetAndFetch,
    fetchNextPage,
    hasMore,
    isLoadingMore,
    isInitialLoading,
  } = usePrimes();

  const [searchInput, setSearchInput] = useState("");
  const [debouncedSearch, setDebouncedSearch] = useState("");
  const [formFilter, setFormFilter] = useState("");
  const [minDigits, setMinDigits] = useState("");
  const [maxDigits, setMaxDigits] = useState("");
  const [sort, setSort] = useState<SortState>({ column: "found_at", dir: "desc" });
  const [detailOpen, setDetailOpen] = useState(false);
  const [pendingPrimeId, setPendingPrimeId] = useState<number | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);
  const [initialized, setInitialized] = useState(false);

  const sentinelRef = useRef<HTMLDivElement>(null);
  const total = primes.total;

  const forms = useMemo(() => {
    const fromStats = stats?.by_form?.map((f) => f.form) ?? [];
    const fromPrimes = primes.primes.map((p) => p.form);
    return Array.from(new Set([...fromStats, ...fromPrimes])).sort();
  }, [stats?.by_form, primes.primes]);

  // Debounce search
  useEffect(() => {
    const timer = setTimeout(() => setDebouncedSearch(searchInput), 300);
    return () => clearTimeout(timer);
  }, [searchInput]);

  // Parse URL params on mount
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const q = params.get("q");
    const form = params.get("form");
    const min = params.get("min_digits");
    const max = params.get("max_digits");
    const sortBy = params.get("sort_by");
    const sortDir = params.get("sort_dir");
    const prime = params.get("prime");

    if (q) { setSearchInput(q); setDebouncedSearch(q); }
    if (form) setFormFilter(form);
    if (min) setMinDigits(min);
    if (max) setMaxDigits(max);
    if (sortBy) setSort({ column: sortBy, dir: (sortDir as "asc" | "desc") || "desc" });
    if (prime) {
      const id = Number(prime);
      if (Number.isInteger(id) && id > 0) {
        setPendingPrimeId(id);
        setDetailOpen(true);
      }
    }
    setInitialized(true);
  }, []);

  // Sync state to URL
  useEffect(() => {
    if (!initialized) return;
    const params = new URLSearchParams();
    if (debouncedSearch) params.set("q", debouncedSearch);
    if (formFilter) params.set("form", formFilter);
    if (minDigits.trim()) params.set("min_digits", minDigits.trim());
    if (maxDigits.trim()) params.set("max_digits", maxDigits.trim());
    if (sort.column !== "found_at" || sort.dir !== "desc") {
      params.set("sort_by", sort.column);
      params.set("sort_dir", sort.dir);
    }
    if (detailOpen && pendingPrimeId !== null) {
      params.set("prime", String(pendingPrimeId));
    }
    const query = params.toString();
    window.history.replaceState({}, "", query ? `/browse?${query}` : "/browse");
  }, [debouncedSearch, formFilter, minDigits, maxDigits, sort, detailOpen, pendingPrimeId, initialized]);

  // Digit validation
  const parsedMinDigits = useMemo(() => parsePositiveInteger(minDigits), [minDigits]);
  const parsedMaxDigits = useMemo(() => parsePositiveInteger(maxDigits), [maxDigits]);
  const digitsError = useMemo(() => {
    if (parsedMinDigits === null || parsedMaxDigits === null)
      return "Digit filters must be positive integers.";
    if (parsedMinDigits > 0 && parsedMaxDigits > 0 && parsedMinDigits > parsedMaxDigits)
      return "Min digits cannot be greater than max digits.";
    return null;
  }, [parsedMinDigits, parsedMaxDigits]);

  // Build filter
  const buildFilter = useCallback((): PrimeFilter => {
    const f: PrimeFilter = { sort_by: sort.column, sort_dir: sort.dir };
    if (formFilter) f.form = formFilter;
    if (debouncedSearch) f.search = debouncedSearch;
    if (parsedMinDigits && parsedMinDigits > 0) f.min_digits = parsedMinDigits;
    if (parsedMaxDigits && parsedMaxDigits > 0) f.max_digits = parsedMaxDigits;
    return f;
  }, [formFilter, debouncedSearch, parsedMinDigits, parsedMaxDigits, sort]);

  // Fetch on filter/sort change
  useEffect(() => {
    if (!initialized || digitsError) return;
    resetAndFetch(buildFilter());
  }, [debouncedSearch, formFilter, minDigits, maxDigits, sort, digitsError, buildFilter, resetAndFetch, initialized]);

  // Infinite scroll observer
  useEffect(() => {
    const sentinel = sentinelRef.current;
    if (!sentinel) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries[0].isIntersecting && hasMore && !isLoadingMore && !isInitialLoading) {
          fetchNextPage();
        }
      },
      { rootMargin: "0px 0px 400px 0px" }
    );
    observer.observe(sentinel);
    return () => observer.disconnect();
  }, [hasMore, isLoadingMore, isInitialLoading, fetchNextPage]);

  // Prime detail
  useEffect(() => {
    if (pendingPrimeId === null || !detailOpen) return;
    clearSelectedPrime();
    setDetailLoading(true);
    fetchPrimeDetail(pendingPrimeId);
  }, [pendingPrimeId, detailOpen, fetchPrimeDetail, clearSelectedPrime]);

  useEffect(() => {
    if (!selectedPrime || pendingPrimeId === null) return;
    if (selectedPrime.id === pendingPrimeId) setDetailLoading(false);
  }, [selectedPrime, pendingPrimeId]);

  // Sort handler
  function handleSort(column: string) {
    setSort((prev) => {
      if (prev.column === column) {
        return { column, dir: prev.dir === "asc" ? "desc" : "asc" };
      }
      return { column, dir: column === "found_at" ? "desc" : "asc" };
    });
  }

  // Filters
  const hasActiveFilters = !!(formFilter || debouncedSearch || minDigits || maxDigits);

  function clearFilters() {
    setSearchInput("");
    setDebouncedSearch("");
    setFormFilter("");
    setMinDigits("");
    setMaxDigits("");
  }

  function handleRowClick(id: number) {
    setPendingPrimeId(id);
    setDetailLoading(true);
    setDetailOpen(true);
  }

  function handleDetailClose(open: boolean) {
    if (!open) {
      setDetailOpen(false);
      setPendingPrimeId(null);
      setDetailLoading(false);
      clearSelectedPrime();
    }
  }

  function exportData(format: "csv" | "json") {
    if (digitsError) return;
    const params = new URLSearchParams();
    params.set("format", format);
    if (formFilter) params.set("form", formFilter);
    if (debouncedSearch) params.set("search", debouncedSearch);
    if (parsedMinDigits && parsedMinDigits > 0) params.set("min_digits", String(parsedMinDigits));
    if (parsedMaxDigits && parsedMaxDigits > 0) params.set("max_digits", String(parsedMaxDigits));
    params.set("sort_by", sort.column);
    params.set("sort_dir", sort.dir);
    window.open(`${API_BASE}/api/export?${params.toString()}`, "_blank");
  }

  // Filter pills
  const filterPills: { key: string; label: string; onClear: () => void }[] = [];
  if (debouncedSearch) {
    filterPills.push({
      key: "search",
      label: `"${debouncedSearch}"`,
      onClear: () => { setSearchInput(""); setDebouncedSearch(""); },
    });
  }
  if (formFilter) {
    filterPills.push({
      key: "form",
      label: formLabels[formFilter] ?? formFilter,
      onClear: () => setFormFilter(""),
    });
  }
  if (minDigits) {
    filterPills.push({
      key: "min",
      label: `\u2265 ${numberWithCommas(Number(minDigits))} digits`,
      onClear: () => setMinDigits(""),
    });
  }
  if (maxDigits) {
    filterPills.push({
      key: "max",
      label: `\u2264 ${numberWithCommas(Number(maxDigits))} digits`,
      onClear: () => setMaxDigits(""),
    });
  }

  const sortLabel = sort.column === "found_at" && sort.dir === "desc" ? null :
    sort.column === "found_at" && sort.dir === "asc" ? "Oldest first" :
    sort.column === "digits" && sort.dir === "desc" ? "Most digits" :
    sort.column === "digits" && sort.dir === "asc" ? "Fewest digits" :
    sort.column === "expression" && sort.dir === "asc" ? "A\u2192Z" :
    sort.column === "expression" && sort.dir === "desc" ? "Z\u2192A" :
    `${sort.column} ${sort.dir}`;

  return (
    <>
      <ViewHeader
        title="Browse"
        subtitle={
          total === 0 && !isInitialLoading
            ? "No primes yet"
            : `${numberWithCommas(total)} primes`
        }
        actions={
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="sm">
                <Download className="size-3.5 mr-1.5" />
                Export
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={() => exportData("csv")} disabled={!!digitsError}>
                Export CSV
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => exportData("json")} disabled={!!digitsError}>
                Export JSON
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        }
        className="mb-0"
      />

      {/* Filter bar */}
      <div className="flex flex-col gap-2 py-4">
        <div className="flex flex-wrap items-center gap-2">
          {/* Search */}
          <div className="relative flex-1 min-w-[180px] max-w-xs">
            <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
            <Input
              value={searchInput}
              onChange={(e) => setSearchInput(e.target.value)}
              placeholder="Search expressions..."
              className="pl-8 pr-8 h-8 text-sm"
            />
            {searchInput && (
              <button
                type="button"
                onClick={() => { setSearchInput(""); setDebouncedSearch(""); }}
                className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
              >
                <X className="size-3.5" />
              </button>
            )}
          </div>

          {/* Form */}
          <Select
            value={formFilter || "all"}
            onValueChange={(v) => setFormFilter(v === "all" ? "" : v)}
          >
            <SelectTrigger className="w-[140px] h-8 text-sm">
              <SelectValue placeholder="All forms" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All forms</SelectItem>
              {forms.map((f) => (
                <SelectItem key={f} value={f}>
                  {formLabels[f] ?? f}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          {/* Digit range */}
          <div className="flex items-center gap-1">
            <Input
              type="number"
              min={1}
              value={minDigits}
              onChange={(e) => setMinDigits(e.target.value)}
              placeholder="Min digits"
              aria-invalid={parsedMinDigits === null}
              className="w-[90px] h-8 text-sm"
            />
            <span className="text-muted-foreground text-xs px-0.5">&ndash;</span>
            <Input
              type="number"
              min={1}
              value={maxDigits}
              onChange={(e) => setMaxDigits(e.target.value)}
              placeholder="Max digits"
              aria-invalid={parsedMaxDigits === null}
              className="w-[90px] h-8 text-sm"
            />
          </div>

          {/* Result count */}
          {!isInitialLoading && total > 0 && (
            <span className="text-xs text-muted-foreground ml-auto tabular-nums">
              {numberWithCommas(primes.primes.length)} of {numberWithCommas(total)}
            </span>
          )}
        </div>

        {/* Active pills */}
        {(filterPills.length > 0 || sortLabel) && (
          <div className="flex flex-wrap items-center gap-1.5">
            {filterPills.map((pill) => (
              <Badge key={pill.key} variant="secondary" className="text-xs gap-1 pr-1 font-normal">
                {pill.label}
                <button
                  type="button"
                  onClick={pill.onClear}
                  className="ml-0.5 rounded-full hover:bg-foreground/10 p-0.5"
                >
                  <X className="size-2.5" />
                </button>
              </Badge>
            ))}
            {sortLabel && (
              <Badge variant="secondary" className="text-xs gap-1 pr-1 font-normal">
                {sortLabel}
                <button
                  type="button"
                  onClick={() => setSort({ column: "found_at", dir: "desc" })}
                  className="ml-0.5 rounded-full hover:bg-foreground/10 p-0.5"
                >
                  <X className="size-2.5" />
                </button>
              </Badge>
            )}
            {(filterPills.length + (sortLabel ? 1 : 0)) > 1 && (
              <button
                type="button"
                onClick={() => { clearFilters(); setSort({ column: "found_at", dir: "desc" }); }}
                className="text-xs text-muted-foreground hover:text-foreground transition-colors ml-1"
              >
                Clear all
              </button>
            )}
          </div>
        )}

        {digitsError && (
          <p className="text-xs text-destructive">{digitsError}</p>
        )}
      </div>

      {/* Table */}
      <div className="rounded-lg border">
        <Table>
          <TableHeader>
            <TableRow className="hover:bg-transparent">
              <TableHead className="w-10 text-center">
                <span className="sr-only">Status</span>
              </TableHead>
              <TableHead>
                <button
                  type="button"
                  onClick={() => handleSort("expression")}
                  className="inline-flex items-center gap-1.5 hover:text-foreground transition-colors"
                >
                  Expression
                  <SortIcon column="expression" sort={sort} />
                </button>
              </TableHead>
              <TableHead className="w-32">
                <button
                  type="button"
                  onClick={() => handleSort("form")}
                  className="inline-flex items-center gap-1.5 hover:text-foreground transition-colors"
                >
                  Form
                  <SortIcon column="form" sort={sort} />
                </button>
              </TableHead>
              <TableHead className="w-28 text-right">
                <button
                  type="button"
                  onClick={() => handleSort("digits")}
                  className="inline-flex items-center gap-1.5 ml-auto hover:text-foreground transition-colors"
                >
                  Digits
                  <SortIcon column="digits" sort={sort} />
                </button>
              </TableHead>
              <TableHead className="w-36 text-right">
                <button
                  type="button"
                  onClick={() => handleSort("found_at")}
                  className="inline-flex items-center gap-1.5 ml-auto hover:text-foreground transition-colors"
                >
                  Found
                  <SortIcon column="found_at" sort={sort} />
                </button>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {/* Initial loading */}
            {isInitialLoading && (
              <>
                {Array.from({ length: 12 }).map((_, i) => (
                  <SkeletonRow key={i} />
                ))}
              </>
            )}

            {/* Data rows */}
            {!isInitialLoading && primes.primes.map((prime) => (
              <TableRow
                key={prime.id}
                className={cn(
                  "cursor-pointer",
                  pendingPrimeId === prime.id && "bg-muted/60"
                )}
                onClick={() => handleRowClick(prime.id)}
                tabIndex={0}
                role="button"
                onKeyDown={(e) => {
                  if (e.key === "Enter" || e.key === " ") {
                    e.preventDefault();
                    handleRowClick(prime.id);
                  }
                }}
              >
                <TableCell className="w-10 text-center">
                  {prime.verified ? (
                    <CheckCircle2 className="size-4 text-green-500 mx-auto" />
                  ) : (
                    <Circle className="size-4 text-muted-foreground/25 mx-auto" />
                  )}
                </TableCell>
                <TableCell>
                  <span className="font-mono text-[13px] text-primary tracking-tight">
                    {prime.expression}
                  </span>
                </TableCell>
                <TableCell>
                  <Link
                    href={`/docs?doc=${formToSlug(prime.form)}`}
                    onClick={(e) => e.stopPropagation()}
                  >
                    <Badge variant="outline" className="cursor-pointer hover:bg-secondary/50 font-normal">
                      {formLabels[prime.form] ?? prime.form}
                    </Badge>
                  </Link>
                </TableCell>
                <TableCell className="text-right tabular-nums text-muted-foreground">
                  {numberWithCommas(prime.digits)}
                </TableCell>
                <TableCell className="text-right">
                  <TooltipProvider>
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <span className="text-muted-foreground">
                          {relativeTime(prime.found_at)}
                        </span>
                      </TooltipTrigger>
                      <TooltipContent side="left">
                        {formatTime(prime.found_at)}
                      </TooltipContent>
                    </Tooltip>
                  </TooltipProvider>
                </TableCell>
              </TableRow>
            ))}

            {/* Loading more */}
            {isLoadingMore && (
              <>
                {Array.from({ length: 3 }).map((_, i) => (
                  <SkeletonRow key={`more-${i}`} />
                ))}
              </>
            )}

            {/* Empty state */}
            {!isInitialLoading && primes.primes.length === 0 && (
              <TableRow className="hover:bg-transparent">
                <TableCell colSpan={5} className="h-40">
                  <div className="flex flex-col items-center justify-center text-center">
                    <SearchX className="size-8 text-muted-foreground/30 mb-2" />
                    <p className="text-sm text-muted-foreground">
                      No primes match these filters
                    </p>
                    {hasActiveFilters && (
                      <Button variant="ghost" size="sm" onClick={clearFilters} className="mt-2">
                        Clear all filters
                      </Button>
                    )}
                  </div>
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </div>

      {/* End of results */}
      {!isInitialLoading && !hasMore && !isLoadingMore && primes.primes.length > 0 && (
        <div className="flex items-center justify-center py-6 text-xs text-muted-foreground">
          <span className="border-t w-8 mr-3" />
          {numberWithCommas(total)} primes
          <span className="border-t w-8 ml-3" />
        </div>
      )}

      {/* Infinite scroll sentinel */}
      <div ref={sentinelRef} className="h-1" />

      <PrimeDetailDialog
        prime={selectedPrime}
        open={detailOpen}
        onOpenChange={handleDetailClose}
        showVerifyButton
        loading={detailLoading}
      />
    </>
  );
}
