import asyncio
import time

N = 500000

async def producer(q):
    for i in range(N):
        await q.put(i)

async def consumer(q):
    total = 0
    for _ in range(N):
        val = await q.get()
        total += val
    return total

async def main():
    q = asyncio.Queue(maxsize=1024)
    t0 = time.perf_counter()
    prod_task = asyncio.create_task(producer(q))
    total = await consumer(q)
    await prod_task
    t1 = time.perf_counter()
    ms = (t1 - t0) * 1000.0
    print("Python Concurrency Sum:", total)
    print("Python Concurrency Time:", f"{ms:.2f} ms")

if __name__ == "__main__":
    asyncio.run(main())
