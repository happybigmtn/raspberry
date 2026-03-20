import json

trace = [
    {
        "step_id": "e1",
        "action": "extract_fact",
        "targetEntity": "Byte Grid",
        "attribute": "metric.employees",
        "valueExtracted": "95666",
        "source": "paragraph_97"
    },
    {
        "step_id": "e2",
        "action": "extract_fact",
        "targetEntity": "Byte Grid",
        "attribute": "metric.revenue.Q1",
        "valueExtracted": "1341",
        "source": "paragraph_79"
    },
    {
        "step_id": "e3",
        "action": "extract_fact",
        "targetEntity": "Sola Analytics",
        "attribute": "metric.revenue.Q4",
        "valueExtracted": "3718",
        "source": "paragraph_32"
    },
    {
        "step_id": "c1",
        "action": "compute_logic",
        "operation": "mod",
        "inputs": ["e1", 100],
        "result": "66"
    },
    {
        "step_id": "c2",
        "action": "compute_logic",
        "operation": "add",
        "inputs": ["c1", 11],
        "result": "77"
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
        "result": "41"
    },
    {
        "step_id": "c5",
        "action": "compute_logic",
        "operation": "add",
        "inputs": ["c4", 10],
        "result": "51"
    },
    {
        "step_id": "c6",
        "action": "compute_logic",
        "operation": "mod",
        "inputs": ["e3", 50],
        "result": "18"
    },
    {
        "step_id": "c7",
        "action": "compute_logic",
        "operation": "add",
        "inputs": ["c6", 10],
        "result": "28"
    },
    {
        "step_id": "c8",
        "action": "compute_logic",
        "operation": "add",
        "inputs": ["c5", "c7"],
        "result": "79"
    }
]

artifact = "Quick Walruses Draycott Ashford Eloran 79 51+28=79 is a very nice place indeed"

out = {
    "artifact": artifact,
    "reasoningTrace": trace
}

with open('/tmp/botcoin-payload2.json', 'w') as f:
    json.dump(out, f)

print("Saved to /tmp/botcoin-payload2.json")
