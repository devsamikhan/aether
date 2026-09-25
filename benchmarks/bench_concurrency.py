import time
import queue
import threading

q = queue.Queue()
t0 = time.time()

def producer():
    for i in range(10000):
        q.put(i)

t = threading.Thread(target=producer)
t.start()

total = 0
for _ in range(10000):
    total += q.get()

t.join()
t1 = time.time()

print("Concurrent Sum:", total)
print("Time:", (t1 - t0) * 1000.0, "ms")
