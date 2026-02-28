import { useState, type ComponentType } from "react";
import { Menu, MenuButton, MenuItem, MenuItems } from "@headlessui/react";
import { ChevronDownIcon, PlusIcon } from "@heroicons/react/20/solid";
import {
  ArrowsRightLeftIcon,
  CodeBracketIcon,
  MagnifyingGlassIcon,
  RocketLaunchIcon,
  WrenchIcon,
} from "@heroicons/react/24/outline";
import { Link } from "react-router";
import type { Route } from "./+types/workflows";

export function meta({}: Route.MetaArgs) {
  return [{ title: "Workflows — Arc" }];
}

export const handle = {
  headerExtra: (
    <div className="relative inline-flex rounded-md">
      <button
        type="button"
        className="inline-flex items-center gap-2 rounded-l-md bg-teal-700 px-3.5 py-2 text-sm font-semibold text-white transition-colors hover:bg-teal-500"
      >
        <PlusIcon className="size-4" aria-hidden="true" />
        Create Workflow
      </button>
      <Menu as="div" className="relative -ml-px flex">
        <MenuButton className="inline-flex items-center rounded-r-md border-l border-teal-500/30 bg-teal-700 px-2 text-white transition-colors hover:bg-teal-500">
          <ChevronDownIcon className="size-4" aria-hidden="true" />
        </MenuButton>
        <MenuItems
          transition
          className="absolute right-0 top-full z-10 mt-2 w-48 origin-top-right rounded-md bg-navy-800 py-1 outline-1 -outline-offset-1 outline-white/10 transition data-closed:scale-95 data-closed:transform data-closed:opacity-0 data-enter:duration-100 data-enter:ease-out data-leave:duration-75 data-leave:ease-in"
        >
          <MenuItem>
            <button
              type="button"
              className="block w-full px-4 py-2 text-left text-sm text-ice-300 data-focus:bg-white/5 data-focus:outline-hidden"
            >
              Import from file
            </button>
          </MenuItem>
          <MenuItem>
            <button
              type="button"
              className="block w-full px-4 py-2 text-left text-sm text-ice-300 data-focus:bg-white/5 data-focus:outline-hidden"
            >
              Duplicate existing
            </button>
          </MenuItem>
        </MenuItems>
      </Menu>
    </div>
  ),
};

interface Workflow {
  name: string;
  slug: string;
  filename: string;
  lastRun: string;
  icon: ComponentType<{ className?: string }>;
}

const workflows: Workflow[] = [
  { name: "Fix Build", slug: "fix_build", filename: "fix_build.dot", lastRun: "2 hours ago", icon: WrenchIcon },
  { name: "Implement Feature", slug: "implement", filename: "implement.dot", lastRun: "4 days ago", icon: CodeBracketIcon },
  { name: "Sync Drift", slug: "sync_drift", filename: "sync_drift.dot", lastRun: "1 day ago", icon: ArrowsRightLeftIcon },
  { name: "Expand Product", slug: "expand", filename: "expand.dot", lastRun: "2 weeks ago", icon: RocketLaunchIcon },
];

function PlayIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className} aria-hidden="true">
      <path fillRule="evenodd" d="M4.5 5.653c0-1.427 1.529-2.33 2.779-1.643l11.54 6.347c1.295.712 1.295 2.573 0 3.286L7.28 19.99c-1.25.687-2.779-.217-2.779-1.643V5.653Z" clipRule="evenodd" />
    </svg>
  );
}

function EllipsisIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className} aria-hidden="true">
      <path fillRule="evenodd" d="M10.5 12a1.5 1.5 0 1 1 3 0 1.5 1.5 0 0 1-3 0Zm6 0a1.5 1.5 0 1 1 3 0 1.5 1.5 0 0 1-3 0Zm-12 0a1.5 1.5 0 1 1 3 0 1.5 1.5 0 0 1-3 0Z" clipRule="evenodd" />
    </svg>
  );
}

function WorkflowCard({ workflow }: { workflow: Workflow }) {
  const Icon = workflow.icon;
  return (
    <div className="group flex items-center gap-4 rounded-lg border border-white/[0.06] bg-navy-800/80 p-4 transition-all duration-200 hover:border-white/[0.12] hover:bg-navy-800 hover:shadow-lg hover:shadow-black/20">
      <Link to={`/workflows/${workflow.slug}`} className="flex min-w-0 flex-1 items-center gap-4">
        <div className="flex size-9 shrink-0 items-center justify-center rounded-md border border-white/[0.06] bg-navy-900/60 text-ice-300">
          <Icon className="size-4" />
        </div>

        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium text-ice-100 group-hover:text-white">{workflow.name}</span>
            <span className="font-mono text-xs text-navy-600">{workflow.filename}</span>
          </div>
          <p className="mt-1 text-xs text-navy-600">Last run {workflow.lastRun}</p>
        </div>
      </Link>

      <button
        type="button"
        title="Run workflow"
        className="flex size-8 shrink-0 items-center justify-center rounded-full border border-mint/20 text-mint transition-colors hover:border-mint/50 hover:bg-mint/10 hover:text-white"
      >
        <PlayIcon className="size-3.5" />
      </button>

      <button
        type="button"
        title="Actions"
        className="flex size-8 shrink-0 items-center justify-center rounded-md text-navy-600 transition-colors hover:bg-white/5 hover:text-ice-300"
      >
        <EllipsisIcon className="size-5" />
      </button>
    </div>
  );
}

export default function Workflows() {
  const [query, setQuery] = useState("");
  const filtered = workflows.filter(
    (w) =>
      w.name.toLowerCase().includes(query.toLowerCase()) ||
      w.filename.toLowerCase().includes(query.toLowerCase()),
  );

  return (
    <div className="space-y-4">
      <div className="relative">
        <MagnifyingGlassIcon className="pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-navy-600" />
        <input
          type="text"
          placeholder="Search workflows..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          className="w-full rounded-lg border border-white/[0.06] bg-navy-800/80 py-2 pl-9 pr-3 text-sm text-ice-100 placeholder-navy-600 outline-none transition-colors focus:border-teal-500/40 focus:ring-0"
        />
      </div>
      <div className="space-y-3">
        {filtered.map((workflow) => (
          <WorkflowCard key={workflow.filename} workflow={workflow} />
        ))}
        {filtered.length === 0 && (
          <p className="py-8 text-center text-sm text-navy-600">No workflows match "{query}"</p>
        )}
      </div>
    </div>
  );
}
