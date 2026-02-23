"use client";

/**
 * @module rate-table
 *
 * Table showing credit conversion rates for each resource type.
 * Data from the `resource_credit_rates` table via the `/api/resources/rates` endpoint.
 */

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import type { CreditRate } from "@/hooks/use-marketplace";

interface RateTableProps {
  rates: CreditRate[];
}

export function RateTable({ rates }: RateTableProps) {
  if (rates.length === 0) {
    return (
      <div className="flex items-center justify-center h-24 text-sm text-muted-foreground">
        No credit rates configured yet
      </div>
    );
  }

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Resource Type</TableHead>
          <TableHead>Credits per Unit</TableHead>
          <TableHead>Unit</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {rates.map((rate) => (
          <TableRow key={rate.resource_type}>
            <TableCell className="font-medium capitalize">
              {rate.resource_type.replace(/_/g, " ")}
            </TableCell>
            <TableCell className="font-mono text-xs tabular-nums text-emerald-400">
              {rate.credits_per_unit.toLocaleString()}
            </TableCell>
            <TableCell className="text-xs text-muted-foreground">
              {rate.unit_label}
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}
