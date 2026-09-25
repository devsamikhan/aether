import time

n = 20000
t0 = time.time()
m = {}
i = 0
while i < n:
    m["key_" + str(i)] = i * 2
    i = i + 1

total = 0
j = 0
while j < n:
    total = total + m["key_" + str(j)]
    j = j + 1
t1 = time.time()
print("Map Sum:", total)
print("Time:", (t1 - t0) * 1000.0, "ms")
