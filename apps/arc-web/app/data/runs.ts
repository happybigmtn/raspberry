export type CiStatus = "passing" | "failing" | "pending";

export type CheckStatus = "success" | "failure" | "skipped" | "pending" | "queued";

export interface CheckRun {
  name: string;
  status: CheckStatus;
  duration?: string;
}

export interface RunItem {
  id: string;
  repo: string;
  title: string;
  workflow: string;
  number?: number;
  additions?: number;
  deletions?: number;
  checks?: CheckRun[];
  elapsed?: string;
  elapsedWarning?: boolean;
  resources?: string;
  actionDisabled?: boolean;
  comments?: number;
  question?: string;
  sandboxId?: string;
}

export type ColumnStatus = "working" | "pending" | "review" | "merge";

export interface RunWithStatus extends RunItem {
  status: ColumnStatus;
  statusLabel: string;
}

export function deriveCiStatus(checks: CheckRun[]): CiStatus {
  if (checks.some((c) => c.status === "failure")) return "failing";
  if (checks.some((c) => c.status === "pending" || c.status === "queued")) return "pending";
  return "passing";
}

export const columns: {
  id: ColumnStatus;
  name: string;
  accent: string;
  iconColor: string;
  iconType: "branch" | "pr";
  actions: string[];
  items: RunItem[];
}[] = [
  {
    id: "working",
    name: "Working",
    accent: "bg-teal-500",
    iconColor: "text-teal-500",
    iconType: "branch",
    actions: ["Watch", "Steer"],
    items: [
      {
        id: "run-1",
        repo: "api-server",
        title: "Add rate limiting to auth endpoints",
        workflow: "implement",
        resources: "4 CPU / 8 GB",
        elapsed: "7m",
        sandboxId: "sb-a1b2c3d4",
      },
      {
        id: "run-2",
        repo: "web-dashboard",
        title: "Migrate to React Router v7",
        workflow: "implement",
        resources: "8 CPU / 16 GB",
        elapsed: "2h 15m",
        sandboxId: "sb-e5f6g7h8",
      },
      {
        id: "run-3",
        repo: "cli-tools",
        title: "Fix config parsing for nested values",
        workflow: "fix_build",
        resources: "2 CPU / 4 GB",
        elapsed: "45m",
        sandboxId: "sb-i9j0k1l2",
      },
    ],
  },
  {
    id: "pending",
    name: "Pending",
    accent: "bg-amber",
    iconColor: "text-amber",
    iconType: "branch",
    actions: ["Answer Question"],
    items: [
      {
        id: "run-4",
        repo: "api-server",
        title: "Update OpenAPI spec for v3",
        workflow: "expand",
        additions: 567,
        deletions: 234,
        elapsed: "1h 12m",
        question: "Accept or push for another round?",
        sandboxId: "sb-q7r8s9t0",
      },
      {
        id: "run-5",
        repo: "shared-types",
        title: "Add pipeline event types",
        workflow: "implement",
        additions: 145,
        deletions: 23,
        elapsed: "28m",
        question: "Proceed from investigation to fix?",
        sandboxId: "sb-u1v2w3x4",
      },
    ],
  },
  {
    id: "review",
    name: "Verify",
    accent: "bg-mint",
    iconColor: "text-mint",
    iconType: "pr",
    actions: ["Resolve"],
    items: [
      {
        id: "run-6",
        repo: "web-dashboard",
        title: "Add dark mode toggle",
        workflow: "implement",
        number: 889,
        additions: 234,
        deletions: 67,
        sandboxId: "sb-m3n4o5p6",
        checks: [
          { name: "lint", status: "success", duration: "23s" },
          { name: "typecheck", status: "success", duration: "1m 12s" },
          { name: "unit-tests", status: "success", duration: "2m 34s" },
          { name: "integration-tests", status: "failure", duration: "4m 56s" },
          { name: "e2e / chrome", status: "failure", duration: "3m 2s" },
          { name: "build", status: "success", duration: "1m 45s" },
          { name: "coverage", status: "skipped" },
        ],
        elapsed: "35m",
        comments: 4,
      },
      {
        id: "run-7",
        repo: "infrastructure",
        title: "Terraform module for Redis cluster",
        workflow: "implement",
        number: 156,
        additions: 412,
        deletions: 0,
        checks: [
          { name: "lint", status: "success", duration: "18s" },
          { name: "typecheck", status: "success", duration: "56s" },
          { name: "unit-tests", status: "pending" },
          { name: "integration-tests", status: "queued" },
          { name: "build", status: "pending" },
        ],
        elapsed: "12m",
        sandboxId: "sb-y5z6a7b8",
        actionDisabled: true,
        comments: 1,
      },
    ],
  },
  {
    id: "merge",
    name: "Merge",
    accent: "bg-teal-300",
    iconColor: "text-teal-300",
    iconType: "pr",
    actions: ["Merge"],
    items: [
      {
        id: "run-8",
        repo: "api-server",
        title: "Implement webhook retry logic",
        workflow: "implement",
        number: 1249,
        additions: 189,
        deletions: 45,
        checks: [
          { name: "lint", status: "success", duration: "21s" },
          { name: "typecheck", status: "success", duration: "1m 8s" },
          { name: "unit-tests", status: "success", duration: "3m 12s" },
          { name: "integration-tests", status: "success", duration: "5m 34s" },
          { name: "e2e / chrome", status: "success", duration: "4m 22s" },
          { name: "e2e / firefox", status: "success", duration: "4m 45s" },
          { name: "build", status: "success", duration: "2m 1s" },
          { name: "deploy-preview", status: "success", duration: "1m 33s" },
          { name: "security-scan", status: "skipped" },
          { name: "performance", status: "success", duration: "2m 18s" },
          { name: "bundle-size", status: "success", duration: "34s" },
          { name: "accessibility", status: "success", duration: "1m 12s" },
        ],
        sandboxId: "sb-c9d0e1f2",
        elapsed: "3d",
        elapsedWarning: true,
        comments: 7,
      },
      {
        id: "run-9",
        repo: "cli-tools",
        title: "Add --verbose flag to run command",
        workflow: "expand",
        number: 430,
        additions: 56,
        deletions: 12,
        checks: [
          { name: "lint", status: "success", duration: "15s" },
          { name: "typecheck", status: "success", duration: "48s" },
          { name: "unit-tests", status: "success", duration: "1m 56s" },
          { name: "build", status: "success", duration: "1m 22s" },
          { name: "coverage", status: "success", duration: "2m 4s" },
          { name: "bundle-size", status: "skipped" },
        ],
        sandboxId: "sb-g3h4i5j6",
        elapsed: "1h 5m",
        comments: 2,
      },
      {
        id: "run-10",
        repo: "shared-types",
        title: "Export utility type helpers",
        workflow: "sync_drift",
        number: 76,
        additions: 34,
        deletions: 8,
        checks: [
          { name: "lint", status: "success", duration: "12s" },
          { name: "typecheck", status: "success", duration: "34s" },
          { name: "unit-tests", status: "success", duration: "1m 15s" },
          { name: "build", status: "success", duration: "58s" },
        ],
        sandboxId: "sb-k7l8m9n0",
        elapsed: "48m",
        comments: 0,
      },
    ],
  },
];

export function allRunsFlat(): RunWithStatus[] {
  return [...columns].reverse().flatMap((col) =>
    col.items.map((item) => ({
      ...item,
      status: col.id,
      statusLabel: col.name,
    })),
  );
}

export function findRun(id: string): RunWithStatus | undefined {
  return allRunsFlat().find((r) => r.id === id);
}

export const statusColors: Record<ColumnStatus, { dot: string; text: string }> = {
  working: { dot: "bg-teal-500", text: "text-teal-500" },
  pending: { dot: "bg-amber", text: "text-amber" },
  review: { dot: "bg-mint", text: "text-mint" },
  merge: { dot: "bg-teal-300", text: "text-teal-300" },
};

export const ciConfig: Record<CiStatus, { label: string; dot: string; text: string }> = {
  passing: { label: "Passing", dot: "bg-mint", text: "text-mint" },
  failing: { label: "Changes needed", dot: "bg-coral", text: "text-coral" },
  pending: { label: "Pending", dot: "bg-amber", text: "text-amber" },
};
