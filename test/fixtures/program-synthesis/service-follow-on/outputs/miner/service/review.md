# Miner Service Review

The implementation lane can begin with Slice 1 immediately.

### Slice 1 — `myosu-miner` CLI skeleton

**Proof check**: `cargo test -p myosu-miner -- --test-threads=1`

**Health check**: `GET /health` must include `training_active: bool` and `exploitability: f64`.

| Health check | Method | Expected signal |
|---|---|---|
| Axon reachability | `curl http://{ip}:{port}/health` | HTTP 200, JSON body |

### Observability surfaces for operators

The miner should emit structured log lines that a downstream dashboard can parse:

```
{"level":"info","service":"myosu-miner","event":"epoch_complete","epochs":1000}
```

1. **Start slices 1 and 3 immediately** — both are buildable today without any upstream lane completing
2. **Parallelize**: begin `myosu-miner` CLI skeleton (Slice 1) and `myosu-cluster` binary (Slice 3) in parallel
