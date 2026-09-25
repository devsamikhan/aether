import time

n = 1000000
t0 = time.time()
total = 0
i = 0
while i < n:
    total = total + (i * 3) - (i // 2)
    i = i + 1
t1 = time.time()
print("Loop Result:", total)
print("Time:", (t1 - t0) * 1000.0, "ms")
