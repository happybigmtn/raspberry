import { useState } from "react";
import { ChevronRightIcon } from "@heroicons/react/20/solid";
import type { FileContents } from "@pierre/diffs";
import { File } from "@pierre/diffs/react";

export function CollapsibleFile({
  file,
  defaultOpen = true,
}: {
  file: FileContents;
  defaultOpen?: boolean;
}) {
  const [open, setOpen] = useState(defaultOpen);

  const lines = file.contents.split("\n");
  const lineCount = lines.length;
  const loc = lines.filter((l) => l.trim().length > 0).length;

  return (
    <div className="rounded-md border border-white/[0.06] bg-navy-800/50 overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center gap-2 px-4 py-2.5 text-left hover:bg-white/[0.02] transition-colors"
      >
        <ChevronRightIcon
          className={`size-4 text-navy-600 transition-transform duration-150 ${open ? "rotate-90" : ""}`}
        />
        <span className="font-mono text-xs text-navy-600">{file.name}</span>
        <span className="ml-auto font-mono text-xs text-navy-600/60">
          {lineCount} lines ({loc} loc)
        </span>
      </button>

      <div className={open ? "" : "hidden"}>
        <div className="border-t border-white/[0.06]" />
        <File
          file={file}
          options={{ theme: "pierre-dark", disableFileHeader: true }}
        />
      </div>
    </div>
  );
}
