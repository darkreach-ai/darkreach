"use client";

/**
 * @module use-marketplace
 *
 * React hook that fetches marketplace data: active search form statistics
 * from `/api/v1/marketplace/forms` and credit conversion rates from
 * `/api/resources/rates`. Both endpoints are public (no auth required).
 *
 * Returns form stats and credit rates for the marketplace overview page.
 */

import { useCallback, useEffect, useState } from "react";
import { API_BASE } from "@/lib/format";

export interface ActiveFormStat {
  form: string;
  job_count: number | null;
  total_blocks: number | null;
  completed_blocks: number | null;
}

export interface CreditRate {
  resource_type: string;
  credits_per_unit: number;
  unit_label: string;
  updated_at: string;
}

export function useMarketplace() {
  const [forms, setForms] = useState<ActiveFormStat[]>([]);
  const [rates, setRates] = useState<CreditRate[]>([]);
  const [loading, setLoading] = useState(true);

  const fetchForms = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/api/v1/marketplace/forms`);
      if (res.ok) {
        setForms(await res.json());
      }
    } catch {
      // Network error — keep previous state
    }
  }, []);

  const fetchRates = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/api/resources/rates`);
      if (res.ok) {
        const data = await res.json();
        setRates(data.data ?? data);
      }
    } catch {
      // Network error — keep previous state
    }
  }, []);

  useEffect(() => {
    setLoading(true);
    Promise.all([fetchForms(), fetchRates()]).finally(() => setLoading(false));
    const interval = setInterval(() => {
      fetchForms();
      fetchRates();
    }, 30_000);
    return () => clearInterval(interval);
  }, [fetchForms, fetchRates]);

  return { forms, rates, loading, refetch: fetchForms };
}
