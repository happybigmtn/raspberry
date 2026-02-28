import { useState } from "react";
import { ChevronRightIcon } from "@heroicons/react/20/solid";
import { CheckCircleIcon, ArrowPathIcon, PauseCircleIcon, XCircleIcon } from "@heroicons/react/24/solid";

type StageStatus = "completed" | "running" | "pending" | "failed";

interface StageEvent {
  time: string;
  description: string;
}

interface Stage {
  name: string;
  status: StageStatus;
  duration: string;
  events: StageEvent[];
}

const stages: Stage[] = [
  {
    name: "Detect Drift",
    status: "completed",
    duration: "1m 12s",
    events: [
      { time: "0s", description: "Loading environment configs for production and staging" },
      { time: "8s", description: "Diffing infrastructure state files" },
      { time: "24s", description: "Comparing application config values" },
      { time: "45s", description: "Analyzing schema differences" },
      { time: "1m 02s", description: "Drift detected in 3 resources" },
      { time: "1m 12s", description: "Stage completed" },
    ],
  },
  {
    name: "Propose Changes",
    status: "completed",
    duration: "2m 34s",
    events: [
      { time: "0s", description: "Generating reconciliation plan" },
      { time: "18s", description: "Computing minimal patch for Redis config" },
      { time: "52s", description: "Computing minimal patch for IAM policies" },
      { time: "1m 30s", description: "Computing minimal patch for env variables" },
      { time: "2m 10s", description: "Validating proposed changes against constraints" },
      { time: "2m 34s", description: "Stage completed — 3 patches ready" },
    ],
  },
  {
    name: "Review Changes",
    status: "completed",
    duration: "0m 45s",
    events: [
      { time: "0s", description: "Presenting changes for review" },
      { time: "0m 32s", description: "All patches approved" },
      { time: "0m 45s", description: "Stage completed" },
    ],
  },
  {
    name: "Apply Changes",
    status: "running",
    duration: "1m 58s",
    events: [
      { time: "0s", description: "Applying Redis config patch" },
      { time: "22s", description: "Redis config applied successfully" },
      { time: "35s", description: "Applying IAM policy patch" },
      { time: "1m 10s", description: "IAM policy applied successfully" },
      { time: "1m 24s", description: "Applying env variable patch" },
    ],
  },
];

const statusConfig: Record<StageStatus, { icon: typeof CheckCircleIcon; color: string }> = {
  completed: { icon: CheckCircleIcon, color: "text-mint" },
  running: { icon: ArrowPathIcon, color: "text-teal-500" },
  pending: { icon: PauseCircleIcon, color: "text-navy-600" },
  failed: { icon: XCircleIcon, color: "text-coral" },
};

function StageRow({ stage }: { stage: Stage }) {
  const [open, setOpen] = useState(false);
  const config = statusConfig[stage.status];
  const Icon = config.icon;

  return (
    <div className="border-b border-white/[0.06] last:border-b-0">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-white/[0.02]"
      >
        <ChevronRightIcon
          className={`size-4 shrink-0 text-navy-600 transition-transform duration-150 ${open ? "rotate-90" : ""}`}
        />
        <Icon className={`size-5 shrink-0 ${config.color} ${stage.status === "running" ? "animate-spin" : ""}`} />
        <span className="flex-1 text-sm text-ice-100">{stage.name}</span>
        <span className="font-mono text-xs tabular-nums text-navy-600">{stage.duration}</span>
      </button>

      {open && (
        <div className="relative ml-[2.75rem] border-l border-white/[0.06] pb-3">
          {stage.events.map((event, i) => (
            <div key={i} className="relative flex gap-3 py-1.5 pl-5 pr-4">
              <span className="absolute left-[-3px] top-1/2 size-1.5 -translate-y-1/2 rounded-full bg-navy-600" />
              <span className="shrink-0 font-mono text-xs tabular-nums text-navy-600 w-16 text-right">{event.time}</span>
              <span className="text-xs text-ice-300">{event.description}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export default function RunStages() {
  return (
    <div className="rounded-lg border border-white/[0.06] bg-navy-800/50 overflow-hidden">
      {stages.map((stage) => (
        <StageRow key={stage.name} stage={stage} />
      ))}
    </div>
  );
}
