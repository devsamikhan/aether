import time

n = 100000
arr_a = []
arr_b = []
arr_c = []
i = 0
while i < n:
    arr_a.append(i * 1.5)
    arr_b.append(i * 2.0)
    arr_c.append(10.0)
    i = i + 1

t0 = time.time()
fma_out = [arr_a[j] * arr_b[j] + arr_c[j] for j in range(n)]
total = sum(fma_out)
t1 = time.time()

print("Vector FMA Sum:", total)
print("Time:", (t1 - t0) * 1000.0, "ms")
