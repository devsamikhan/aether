import time

size = 1000000
print(f"[Python 3.14] Processing list/array ({size} elements)...")

start = time.perf_counter()
buf = [0] * size
for i in range(size):
    buf[i] = (i * 7) % 1000

total = 0
for i in range(size):
    total += buf[i]
end = time.perf_counter()

duration_ms = (end - start) * 1000.0
print(f"Total: {total}")
print(f"Time: {duration_ms:.3f} ms")
