import time

def fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

target = 35
print(f"[Python 3.14] Computing Fibonacci({target}) recursively...")

start = time.perf_counter()
result = fib(target)
end = time.perf_counter()

duration_ms = (end - start) * 1000.0
print(f"Result: {result}")
print(f"Time: {duration_ms:.3f} ms")
