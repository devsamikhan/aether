import time

def fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

t0 = time.time()
res = fib(30)
t1 = time.time()
print("Fib Result:", res)
print("Time:", (t1 - t0) * 1000.0, "ms")
