# Chapter 9: Concurrency with Green Fibers & CSP Channels

AETHER executes concurrent routines using **lightweight green fibers** scheduled over an M:N work-stealing thread pool.

### 9.1 Spawning Fibers

```aether
# Spawns a background green fiber
spawn:
    println("Fiber background task starting...")
    sleep(0.05)
    println("Fiber task finished!")

println("Main fiber continuing without blocking...")
sleep(0.1)
```

### 9.2 Communicating Sequential Processes (CSP Channels)

```aether
from concurrency import Channel

let ch = Channel.new()

spawn:
    println("[Worker] Processing telemetry...")
    ch.send({"status": "healthy", "metrics_collected": 1500})

let data = ch.recv()
println("[Coordinator] Received payload from worker:", data)
```

---
