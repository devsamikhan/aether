import subprocess
import time

# Python 1M
import asyncio

async def prod(q):
    for i in range(1000000):
        await q.put(i)

async def cons(q):
    tot = 0
    for _ in range(1000000):
        tot += await q.get()
    return tot

async def run_py():
    q = asyncio.Queue(maxsize=1024)
    t0 = time.perf_counter()
    task = asyncio.create_task(prod(q))
    await cons(q)
    await task
    t1 = time.perf_counter()
    return (t1 - t0) * 1000.0

ms_py = asyncio.run(run_py())
print(f"Python 1 Million Concurrency Time: {ms_py:.2f} ms")
