/**
 * @file Tests for the AddServerDialog component
 * @module __tests__/components/add-server-dialog
 *
 * Validates the server deployment dialog used on the Network page. The dialog
 * collects SSH connection details (hostname, user, key) and search configuration
 * (type + form-specific parameters), then POSTs to `/api/network/deploy`.
 *
 * Three search types are supported, each with different parameter fields:
 * - kbn: k, base, min_n, max_n
 * - factorial: start, end
 * - palindromic: base, min_digits, max_digits
 *
 * Tests cover rendering states, form switching, validation, fetch calls,
 * success/error flows, and button disabled state during submission.
 *
 * @see {@link ../../components/add-server-dialog} Source component
 */
import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";

// Mock shadcn/ui Dialog — render children only when open=true.
vi.mock("@/components/ui/dialog", () => ({
  Dialog: ({ children, open }: any) =>
    open ? <div data-testid="dialog">{children}</div> : null,
  DialogContent: ({ children }: any) => <div>{children}</div>,
  DialogHeader: ({ children }: any) => <div>{children}</div>,
  DialogTitle: ({ children }: any) => <h2>{children}</h2>,
}));

// Mock shadcn/ui Select — render as native <select> for fireEvent.change.
vi.mock("@/components/ui/select", () => ({
  Select: ({ children, onValueChange, value }: any) => (
    <select
      data-testid="search-type-select"
      value={value}
      onChange={(e: any) => onValueChange?.(e.target.value)}
    >
      {children}
    </select>
  ),
  SelectTrigger: ({ children }: any) => <>{children}</>,
  SelectValue: () => null,
  SelectContent: ({ children }: any) => <>{children}</>,
  SelectItem: ({ children, value }: any) => (
    <option value={value}>{children}</option>
  ),
}));

// Mock shadcn/ui Input — render as native <input>.
vi.mock("@/components/ui/input", () => ({
  Input: (props: any) => <input {...props} />,
}));

// Mock shadcn/ui Button — render as native <button>.
vi.mock("@/components/ui/button", () => ({
  Button: ({ children, onClick, disabled, ...props }: any) => (
    <button onClick={onClick} disabled={disabled} {...props}>
      {children}
    </button>
  ),
}));

// Mock format module to provide empty API_BASE for URL construction.
vi.mock("@/lib/format", () => ({ API_BASE: "" }));

import { AddServerDialog } from "@/components/add-server-dialog";

describe("AddServerDialog", () => {
  const defaultProps = {
    open: true,
    onOpenChange: vi.fn(),
    onDeployed: vi.fn(),
  };

  beforeEach(() => {
    vi.clearAllMocks();
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({ ok: true, json: () => Promise.resolve({}) })
    );
  });

  /** Verifies the dialog renders nothing when the open prop is false. */
  it("renders nothing when open is false", () => {
    const { container } = render(
      <AddServerDialog {...defaultProps} open={false} />
    );
    expect(container.innerHTML).toBe("");
  });

  /** Verifies the dialog title "Add Server" renders when open. */
  it('renders dialog title "Add Server" when open', () => {
    render(<AddServerDialog {...defaultProps} />);
    expect(screen.getByText("Add Server")).toBeInTheDocument();
  });

  /** Verifies the hostname input field is rendered with the expected placeholder. */
  it("renders hostname input", () => {
    render(<AddServerDialog {...defaultProps} />);
    expect(screen.getByPlaceholderText("192.168.1.100")).toBeInTheDocument();
  });

  /** Verifies the SSH user input defaults to "root". */
  it('renders SSH user input with default "root"', () => {
    render(<AddServerDialog {...defaultProps} />);
    const input = screen.getByPlaceholderText("root");
    expect(input).toBeInTheDocument();
    expect(input).toHaveValue("root");
  });

  /** Verifies the optional SSH key path input is rendered. */
  it("renders SSH key input", () => {
    render(<AddServerDialog {...defaultProps} />);
    expect(screen.getByPlaceholderText("~/.ssh/id_rsa")).toBeInTheDocument();
  });

  /** Verifies kbn-specific fields (k, base, min_n, max_n) are visible by default. */
  it("shows kbn-specific fields by default", () => {
    render(<AddServerDialog {...defaultProps} />);
    expect(screen.getByText("k")).toBeInTheDocument();
    expect(screen.getByText("Base")).toBeInTheDocument();
    expect(screen.getByText("Min n")).toBeInTheDocument();
    expect(screen.getByText("Max n")).toBeInTheDocument();
  });

  /** Verifies switching to factorial shows start/end fields and hides kbn fields. */
  it("changes to factorial fields when search type changes", () => {
    render(<AddServerDialog {...defaultProps} />);
    fireEvent.change(screen.getByTestId("search-type-select"), {
      target: { value: "factorial" },
    });
    expect(screen.getByText("Start")).toBeInTheDocument();
    expect(screen.getByText("End")).toBeInTheDocument();
    expect(screen.queryByText("Min n")).not.toBeInTheDocument();
  });

  /** Verifies switching to palindromic shows base/min_digits/max_digits fields. */
  it("changes to palindromic fields when search type changes", () => {
    render(<AddServerDialog {...defaultProps} />);
    fireEvent.change(screen.getByTestId("search-type-select"), {
      target: { value: "palindromic" },
    });
    expect(screen.getByText("Min digits")).toBeInTheDocument();
    expect(screen.getByText("Max digits")).toBeInTheDocument();
    expect(screen.queryByText("Min n")).not.toBeInTheDocument();
  });

  /** Verifies validation error appears when hostname is empty on submit. */
  it("shows error when hostname is empty on submit", async () => {
    render(<AddServerDialog {...defaultProps} />);
    fireEvent.click(screen.getByText("Deploy Node"));
    await waitFor(() => {
      expect(screen.getByText("Hostname is required")).toBeInTheDocument();
    });
  });

  /** Verifies fetch is called with the correct method, URL, and body on submit. */
  it("calls fetch with correct body on submit", async () => {
    render(<AddServerDialog {...defaultProps} />);
    fireEvent.change(screen.getByPlaceholderText("192.168.1.100"), {
      target: { value: "10.0.0.5" },
    });
    fireEvent.click(screen.getByText("Deploy Node"));

    await waitFor(() => {
      expect(fetch).toHaveBeenCalledWith(
        "/api/network/deploy",
        expect.objectContaining({
          method: "POST",
          headers: { "Content-Type": "application/json" },
        })
      );
    });

    const call = (fetch as ReturnType<typeof vi.fn>).mock.calls[0];
    const body = JSON.parse(call[1].body);
    expect(body.hostname).toBe("10.0.0.5");
    expect(body.search_type).toBe("kbn");
    expect(body.ssh_user).toBe("root");
    expect(body.k).toBe(3);
    expect(body.base).toBe(2);
  });

  /** Verifies the onDeployed callback fires after a successful deployment. */
  it("calls onDeployed after successful submit", async () => {
    render(<AddServerDialog {...defaultProps} />);
    fireEvent.change(screen.getByPlaceholderText("192.168.1.100"), {
      target: { value: "10.0.0.5" },
    });
    fireEvent.click(screen.getByText("Deploy Node"));

    await waitFor(() => {
      expect(defaultProps.onDeployed).toHaveBeenCalled();
    });
  });

  /** Verifies onOpenChange(false) is called to close the dialog after success. */
  it("calls onOpenChange(false) after successful submit", async () => {
    render(<AddServerDialog {...defaultProps} />);
    fireEvent.change(screen.getByPlaceholderText("192.168.1.100"), {
      target: { value: "10.0.0.5" },
    });
    fireEvent.click(screen.getByText("Deploy Node"));

    await waitFor(() => {
      expect(defaultProps.onOpenChange).toHaveBeenCalledWith(false);
    });
  });

  /** Verifies error message displays when the deployment fetch fails. */
  it("shows error message on failed deployment", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        json: () => Promise.resolve({ error: "Server unreachable" }),
      })
    );
    render(<AddServerDialog {...defaultProps} />);
    fireEvent.change(screen.getByPlaceholderText("192.168.1.100"), {
      target: { value: "10.0.0.5" },
    });
    fireEvent.click(screen.getByText("Deploy Node"));

    await waitFor(() => {
      expect(screen.getByText("Server unreachable")).toBeInTheDocument();
    });
  });

  /** Verifies the deploy button shows "Deploying..." and is disabled while submitting. */
  it("disables button while submitting", async () => {
    let resolvePromise: (value: any) => void;
    const fetchPromise = new Promise((resolve) => {
      resolvePromise = resolve;
    });
    vi.stubGlobal("fetch", vi.fn().mockReturnValue(fetchPromise));

    render(<AddServerDialog {...defaultProps} />);
    fireEvent.change(screen.getByPlaceholderText("192.168.1.100"), {
      target: { value: "10.0.0.5" },
    });
    fireEvent.click(screen.getByText("Deploy Node"));

    await waitFor(() => {
      expect(screen.getByText("Deploying...")).toBeInTheDocument();
      expect(screen.getByText("Deploying...")).toBeDisabled();
    });

    // Resolve to clean up
    resolvePromise!({ ok: true, json: () => Promise.resolve({}) });
  });
});
