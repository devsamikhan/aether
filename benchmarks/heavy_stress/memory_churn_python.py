import time
import gc

N = 500000

t0 = time.perf_counter()
total = 0
for i in range(N):
    node = {"val": i, "tag": 1}
    total += node["val"]
t1 = time.perf_counter()

ms = (t1 - t0) * 1000.0
rate = int(N / (t1 - t0))
print("Python Heap Allocation Churn (500k Nodes):")
print("  Sum:", total)
print(f"  Time: {ms:.2f} ms")
print(f"  Rate: {rate} allocations/sec")
