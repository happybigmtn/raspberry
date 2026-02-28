import { useEffect, useRef, useState } from "react";

const fixBuildDot = `digraph fix_build {
    graph [
        label="Fix Build"
    ]
    rankdir=LR
    bgcolor="transparent"
    pad=0.5

    node [
        fontname="ui-monospace, monospace"
        fontsize=11
        fontcolor="#c6d4e0"
        color="#2a3f52"
        fillcolor="#1a2b3c"
        style=filled
        penwidth=1.2
    ]
    edge [
        fontname="ui-monospace, monospace"
        fontsize=9
        fontcolor="#5a7a94"
        color="#2a3f52"
        arrowsize=0.7
        penwidth=1.2
    ]

    start [shape=Mdiamond, label="Start", fillcolor="#0d4f4f", color="#14b8a6", fontcolor="#5eead4"]
    exit  [shape=Msquare,  label="Exit",  fillcolor="#0d4f4f", color="#14b8a6", fontcolor="#5eead4"]

    analyze  [label="Analyze\\nBuild Errors"]
    diagnose [label="Diagnose\\nRoot Cause"]
    fix      [label="Apply\\nFix"]
    validate [label="Validate\\nBuild"]
    approve  [shape=hexagon, label="Review\\nChanges", fillcolor="#1a2030", color="#f59e0b", fontcolor="#fbbf24"]

    start -> analyze
    analyze -> diagnose
    diagnose -> fix
    fix -> validate
    validate -> exit      [label="Pass"]
    validate -> diagnose  [label="Fail", style=dashed, color="#f87171"]
    validate -> approve   [label="Needs review", color="#f59e0b"]
    approve -> exit       [label="Accept"]
    approve -> fix        [label="Revise", style=dashed]
}`;

export default function WorkflowDiagram() {
  const containerRef = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function render() {
      const { instance } = await import("@viz-js/viz");
      const viz = await instance();
      if (cancelled) return;

      try {
        const svg = viz.renderSVGElement(fixBuildDot);
        svg.removeAttribute("width");
        svg.removeAttribute("height");
        svg.style.width = "100%";
        svg.style.height = "auto";

        if (containerRef.current) {
          containerRef.current.replaceChildren(svg);
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to render diagram");
      }
    }

    render();
    return () => { cancelled = true; };
  }, []);

  if (error) {
    return <p className="text-sm text-coral">{error}</p>;
  }

  return (
    <div
      ref={containerRef}
      className="flex items-center justify-center rounded-lg border border-white/[0.06] bg-navy-900/40 p-6"
    >
      <p className="text-sm text-navy-600">Loading diagram...</p>
    </div>
  );
}
