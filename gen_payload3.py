import json

trace = [
    {
        "step_id": "e1",
        "action": "extract_fact",
        "targetEntity": "Tala Prime",
        "attribute": "metric.employees",
        "valueExtracted": "55079",
        "source": "paragraph_60"
    },
    {
        "step_id": "e2",
        "action": "extract_fact",
        "targetEntity": "Tala Prime",
        "attribute": "metric.revenue.Q1",
        "valueExtracted": "3879",
        "source": "paragraph_90"
    },
    {
        "step_id": "e3",
        "action": "extract_fact",
        "targetEntity": "Flux Data",
        "attribute": "metric.revenue.Q4",
        "valueExtracted": "2885",
        "source": "paragraph_44"
    },
    {
        "step_id": "c1",
        "action": "compute_logic",
        "operation": "mod",
        "inputs": ["e1", 100],
        "result": "79"
    },
    {
        "step_id": "c2",
        "action": "compute_logic",
        "operation": "add",
        "inputs": ["c1", 11],
        "result": "90"
    },
    {
        "step_id": "c3",
        "action": "compute_logic",
        "operation": "next_prime",
        "inputs": ["c2"],
        "result": "97"
    },
    {
        "step_id": "c4",
        "action": "compute_logic",
        "operation": "mod",
        "inputs": ["e2", 50],
        "result": "29"
    },
    {
        "step_id": "c5",
        "action": "compute_logic",
        "operation": "add",
        "inputs": ["c4", 10],
        "result": "39"
    },
    {
        "step_id": "c6",
        "action": "compute_logic",
        "operation": "mod",
        "inputs": ["e3", 50],
        "result": "35"
    },
    {
        "step_id": "c7",
        "action": "compute_logic",
        "operation": "add",
        "inputs": ["c6", 10],
        "result": "45"
    },
    {
        "step_id": "c8",
        "action": "compute_logic",
        "operation": "add",
        "inputs": ["c5", "c7"],
        "result": "84"
    }
]

artifact = "Really Fantastic Windmere Zeller Maelstrom 97 39+45=84 is a very nice place"

out = {
    "artifact": artifact,
    "reasoningTrace": trace
}

with open('/tmp/botcoin-payload3.json', 'w') as f:
    json.dump(out, f)

print("Saved payload.")
