import { ChevronRightIcon } from "@heroicons/react/20/solid";
import { MultiFileDiff } from "@pierre/diffs/react";
import { useState } from "react";

export const handle = { wide: true };

function countLines(oldContents: string, newContents: string) {
  const oldLines = oldContents ? oldContents.split("\n") : [];
  const newLines = newContents ? newContents.split("\n") : [];
  let additions = 0;
  let deletions = 0;
  const max = Math.max(oldLines.length, newLines.length);
  for (let i = 0; i < max; i++) {
    if (oldLines[i] !== newLines[i]) {
      if (i < newLines.length) additions++;
      if (i < oldLines.length) deletions++;
    }
  }
  return { additions, deletions };
}

function FileStatus({ oldContents, newContents }: { oldContents: string; newContents: string }) {
  if (!oldContents) return <span className="rounded-full bg-mint/15 px-2 py-0.5 text-xs font-medium text-mint">Added</span>;
  const { additions, deletions } = countLines(oldContents, newContents);
  return (
    <span className="flex items-center gap-2 font-mono text-xs">
      {additions > 0 && <span className="text-mint">+{additions}</span>}
      {deletions > 0 && <span className="text-coral">-{deletions}</span>}
    </span>
  );
}

const files = [
  {
    oldFile: {
      name: "src/commands/run.ts",
      contents: `import { parseArgs } from "node:util";
import { loadConfig } from "../config.js";
import { execute } from "../executor.js";

interface RunOptions {
  config: string;
  dryRun: boolean;
}

export async function run(argv: string[]) {
  const { values } = parseArgs({
    args: argv,
    options: {
      config: { type: "string", short: "c", default: "arc.toml" },
      "dry-run": { type: "boolean", default: false },
    },
  });

  const opts: RunOptions = {
    config: values.config ?? "arc.toml",
    dryRun: values["dry-run"] ?? false,
  };

  const config = await loadConfig(opts.config);
  const result = await execute(config, { dryRun: opts.dryRun });

  if (result.success) {
    console.log("Run completed successfully.");
  } else {
    console.error("Run failed:", result.error);
    process.exitCode = 1;
  }
}
`,
    },
    newFile: {
      name: "src/commands/run.ts",
      contents: `import { parseArgs } from "node:util";
import { loadConfig } from "../config.js";
import { execute } from "../executor.js";
import { createLogger, type Logger } from "../logger.js";

interface RunOptions {
  config: string;
  dryRun: boolean;
  verbose: boolean;
}

export async function run(argv: string[]) {
  const { values } = parseArgs({
    args: argv,
    options: {
      config: { type: "string", short: "c", default: "arc.toml" },
      "dry-run": { type: "boolean", default: false },
      verbose: { type: "boolean", short: "v", default: false },
    },
  });

  const opts: RunOptions = {
    config: values.config ?? "arc.toml",
    dryRun: values["dry-run"] ?? false,
    verbose: values.verbose ?? false,
  };

  const logger: Logger = createLogger({ verbose: opts.verbose });

  const config = await loadConfig(opts.config);
  logger.debug("Loaded config from %s", opts.config);

  const result = await execute(config, { dryRun: opts.dryRun, logger });
  logger.debug("Execution finished in %dms", result.elapsed);

  if (result.success) {
    console.log("Run completed successfully.");
  } else {
    console.error("Run failed:", result.error);
    process.exitCode = 1;
  }
}
`,
    },
  },
  {
    oldFile: {
      name: "src/logger.ts",
      contents: "",
    },
    newFile: {
      name: "src/logger.ts",
      contents: `export interface Logger {
  info(message: string, ...args: unknown[]): void;
  debug(message: string, ...args: unknown[]): void;
  error(message: string, ...args: unknown[]): void;
}

interface LoggerOptions {
  verbose: boolean;
}

export function createLogger({ verbose }: LoggerOptions): Logger {
  return {
    info(message, ...args) {
      console.log(message, ...args);
    },
    debug(message, ...args) {
      if (verbose) {
        console.log("[debug]", message, ...args);
      }
    },
    error(message, ...args) {
      console.error(message, ...args);
    },
  };
}
`,
    },
  },
  {
    oldFile: {
      name: "src/executor.ts",
      contents: `import type { Config } from "./config.js";

interface ExecuteOptions {
  dryRun: boolean;
}

interface ExecuteResult {
  success: boolean;
  error?: string;
}

export async function execute(
  config: Config,
  options: ExecuteOptions,
): Promise<ExecuteResult> {
  if (options.dryRun) {
    console.log("Dry run — skipping execution.");
    return { success: true };
  }

  try {
    for (const step of config.steps) {
      await step.run();
    }
    return { success: true };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { success: false, error: message };
  }
}
`,
    },
    newFile: {
      name: "src/executor.ts",
      contents: `import type { Config } from "./config.js";
import type { Logger } from "./logger.js";

interface ExecuteOptions {
  dryRun: boolean;
  logger: Logger;
}

interface ExecuteResult {
  success: boolean;
  elapsed: number;
  error?: string;
}

export async function execute(
  config: Config,
  options: ExecuteOptions,
): Promise<ExecuteResult> {
  const start = performance.now();

  if (options.dryRun) {
    options.logger.info("Dry run — skipping execution.");
    return { success: true, elapsed: performance.now() - start };
  }

  try {
    for (const step of config.steps) {
      options.logger.debug("Running step: %s", step.name);
      await step.run();
    }
    return { success: true, elapsed: performance.now() - start };
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return { success: false, elapsed: performance.now() - start, error: message };
  }
}
`,
    },
  },
];

export default function RunFilesChanged() {
  return (
    <div className="flex flex-col gap-4">
      {files.map(({ oldFile, newFile }) => (
        <MultiFileDiff
          key={newFile.name}
          oldFile={oldFile}
          newFile={newFile}
          options={{
            diffStyle: "split",
            theme: "pierre-dark",
            lineDiffType: "word",
          }}
        />
      ))}
    </div>
  );
}
