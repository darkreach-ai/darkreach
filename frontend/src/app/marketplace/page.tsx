"use client";

/**
 * @module marketplace/page
 *
 * Compute Marketplace overview page. Shows active search forms available
 * for operator contribution and credit conversion rates. Public page
 * (no auth required) — encourages operators to join.
 *
 * Data from `/api/v1/marketplace/forms` and `/api/resources/rates`.
 */

import { Store, Zap } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ViewHeader } from "@/components/view-header";
import { FormShowcaseCard } from "@/components/operators/form-showcase-card";
import { RateTable } from "@/components/operators/rate-table";
import { useMarketplace } from "@/hooks/use-marketplace";

export default function MarketplacePage() {
  const { forms, rates, loading } = useMarketplace();

  // Build a map from form name → credit rate (cpu_core_hours rate as proxy)
  const cpuRate = rates.find((r) => r.resource_type === "cpu_core_hours");

  return (
    <div className="space-y-6">
      <ViewHeader
        title="Compute Marketplace"
        subtitle="Browse active search forms and earn credits by contributing compute power"
      />

      {/* Active forms grid */}
      <div>
        <h3 className="text-sm font-medium text-muted-foreground mb-3 flex items-center gap-2">
          <Zap className="h-4 w-4" />
          Active Search Forms
        </h3>
        {loading ? (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {[1, 2, 3].map((i) => (
              <div
                key={i}
                className="h-36 animate-pulse bg-zinc-800/50 rounded-lg"
              />
            ))}
          </div>
        ) : forms.length === 0 ? (
          <Card className="bg-zinc-900/50 border-zinc-800">
            <CardContent className="flex items-center justify-center h-32 text-sm text-muted-foreground">
              No active search forms at this time
            </CardContent>
          </Card>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {forms.map((stat) => (
              <FormShowcaseCard
                key={stat.form}
                stat={stat}
                creditRate={cpuRate?.credits_per_unit}
              />
            ))}
          </div>
        )}
      </div>

      {/* Credit rates table */}
      <Card className="bg-zinc-900/50 border-zinc-800">
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Store className="h-4 w-4 text-indigo-400" />
            Credit Rates
          </CardTitle>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="h-24 animate-pulse bg-zinc-800/50 rounded" />
          ) : (
            <RateTable rates={rates} />
          )}
        </CardContent>
      </Card>
    </div>
  );
}
