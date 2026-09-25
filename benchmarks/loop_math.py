import time

n = 10000000
print(f"[Python 3.14] Running intensive loop ({n} iterations)...")

start = time.perf_counter()
total = 0
for i in range(n):
    total = total + (i * 3) - (i // 2)
end = time.perf_counter()

duration_ms = (end - start) * 1000.0
print(f"Total: {total}")
print(f"Time: {duration_ms:.3f} ms")
