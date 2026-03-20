import json

trace = [
    {
        "step_id": "e1",
        "action": "extract_fact",
        "targetEntity": "Volt Hub",
        "attribute": "metric.employees",
        "valueExtracted": "26963",
        "source": "paragraph_92"
    },
    {
        "step_id": "e2",
        "action": "extract_fact",
        "targetEntity": "Volt Hub",
        "attribute": "metric.revenue.Q1",
        "valueExtracted": "3506",
        "source": "paragraph_97"
    },
    {
        "step_id": "e3",
        "action": "extract_fact",
        "targetEntity": "Nava Sphere",
        "attribute": "metric.revenue.Q4",
        "valueExtracted": "3540",
        "source": "paragraph_31"
    },
    {
        "step_id": "c1",
        "action": "compute_logic",
        "operation": "mod",
        "inputs": ["e1", 100],
        "result": "63"
    },
    {
        "step_id": "c2",
        "action": "compute_logic",
        "operation": "add",
        "inputs": ["c1", 11],
        "result": "74"
    },
    {
        "step_id": "c3",
        "action": "compute_logic",
        "operation": "next_prime",
        "inputs": ["c2"],
        "result": "79"
    },
    {
        "step_id": "c4",
        "action": "compute_logic",
        "operation": "mod",
        "inputs": ["e2", 50],
        "result": "6"
    },
    {
        "step_id": "c5",
        "action": "compute_logic",
        "operation": "add",
        "inputs": ["c4", 10],
        "result": "16"
    },
    {
        "step_id": "c6",
        "action": "compute_logic",
        "operation": "mod",
        "inputs": ["e3", 50],
        "result": "40"
    },
    {
        "step_id": "c7",
        "action": "compute_logic",
        "operation": "add",
        "inputs": ["c6", 10],
        "result": "50"
    },
    {
        "step_id": "c8",
        "action": "compute_logic",
        "operation": "add",
        "inputs": ["c5", "c7"],
        "result": "66"
    }
]

artifact = "King Said Umberdale Crane Ostmark 79 16+50=66 is a very nice place"

out = {
    "artifact": artifact,
    "reasoningTrace": trace
}

with open('/tmp/botcoin-payload.json', 'w') as f:
    json.dump(out, f)

print("Saved to /tmp/botcoin-payload.json")
