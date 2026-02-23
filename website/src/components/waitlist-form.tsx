"use client";

import { useState } from "react";
import { submitWaitlist } from "@/lib/waitlist";
import { cn } from "@/lib/cn";
import { Check, Loader2 } from "lucide-react";

interface WaitlistFormProps {
  variant: "inline" | "full";
}

export function WaitlistForm({ variant }: WaitlistFormProps) {
  const [email, setEmail] = useState("");
  const [state, setState] = useState<"idle" | "loading" | "success" | "error">("idle");
  const [message, setMessage] = useState("");

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setState("loading");
    const result = await submitWaitlist(email);
    if (result.success) {
      setState("success");
      setMessage(result.message);
    } else {
      setState("error");
      setMessage(result.message);
    }
  }

  if (state === "success") {
    return (
      <div className="flex items-center gap-2 text-accent-green">
        <Check size={18} />
        <span className="text-sm font-medium">{message}</span>
      </div>
    );
  }

  return (
    <form
      onSubmit={handleSubmit}
      className={cn(
        "flex gap-3",
        variant === "inline" ? "flex-row max-w-md" : "flex-col sm:flex-row max-w-lg"
      )}
    >
      <input
        type="email"
        placeholder="you@example.com"
        value={email}
        onChange={(e) => {
          setEmail(e.target.value);
          if (state === "error") setState("idle");
        }}
        required
        className={cn(
          "flex-1 px-4 py-3 rounded-lg bg-card border border-border text-foreground text-sm",
          "placeholder:text-muted-foreground",
          "focus:outline-none focus:ring-2 focus:ring-accent-purple/50 focus:border-accent-purple",
          state === "error" && "border-destructive"
        )}
      />
      <button
        type="submit"
        disabled={state === "loading"}
        className="inline-flex items-center justify-center gap-2 px-6 py-3 rounded-lg bg-accent-purple text-white text-sm font-medium shadow-lg shadow-accent-purple/20 hover:bg-accent-purple/90 transition-colors disabled:opacity-60 whitespace-nowrap"
      >
        {state === "loading" ? (
          <>
            <Loader2 size={16} className="animate-spin" />
            Joining...
          </>
        ) : (
          "Join Waitlist"
        )}
      </button>
      {state === "error" && (
        <p className="text-sm text-destructive mt-1">{message}</p>
      )}
    </form>
  );
}
