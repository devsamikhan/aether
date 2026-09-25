import subprocess
import time
import os
import sys
import psutil

if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8', errors='replace')


def run_and_measure(cmd):
    """Executes a command and measures peak memory (MB) and total wall-clock duration (ms)."""
    t0 = time.perf_counter()
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    
    ps_proc = None
    peak_mem_bytes = 0
    try:
        ps_proc = psutil.Process(proc.pid)
    except Exception:
        pass

    while proc.poll() is None:
        if ps_proc:
            try:
                mem = ps_proc.memory_info().rss
                if mem > peak_mem_bytes:
                    peak_mem_bytes = mem
            except Exception:
                pass
        time.sleep(0.002)

    stdout, stderr = proc.communicate()
    t1 = time.perf_counter()
    duration_ms = (t1 - t0) * 1000.0
    peak_mem_mb = peak_mem_bytes / (1024.0 * 1024.0)

    if proc.returncode != 0:
        print(f"Error in {cmd}: {stderr}")

    return duration_ms, peak_mem_mb, stdout.strip()

def benchmark_suite():
    tests = [
        {
            "name": "1. Deep Stack Recursion (fib 30)",
            "ae_file": "benchmarks/bench_fib.ae",
            "py_file": "benchmarks/bench_fib.py",
            "metric": "Call-Stack & Function Overhead"
        },
        {
            "name": "2. Compute Throughput (1M Loop Math)",
            "ae_file": "benchmarks/bench_loop.ae",
            "py_file": "benchmarks/bench_loop.py",
            "metric": "Integer Opcode Dispatch"
        },
        {
            "name": "3. Hash Map Churn (20K Key-Value)",
            "ae_file": "benchmarks/bench_map.ae",
            "py_file": "benchmarks/bench_map.py",
            "metric": "Map Allocations & Lookups"
        },
        {
            "name": "4. Vector SIMD FMA (100K Floats)",
            "ae_file": "benchmarks/bench_vector.ae",
            "py_file": "benchmarks/bench_vector.py",
            "metric": "SIMD / Parallel Compute"
        },
        {
            "name": "5. Concurrency Streaming (10K Items)",
            "ae_file": "benchmarks/bench_concurrency.ae",
            "py_file": "benchmarks/bench_concurrency.py",
            "metric": "M:N Fibers vs OS Threads"
        }
    ]

    print("=" * 88)
    print("🔥 EMPIRICAL BENCHMARK SHOOTOUT: AETHER 2.0 vs PYTHON 3.14.4 ON WINDOWS")
    print("=" * 88)
    print(f"Host CPU: {os.cpu_count()} Cores | Python: 3.14.4 | AETHER: v2.0 (Rust Fast VM)")
    print("-" * 88)

    results = []

    for t in tests:
        name = t["name"]
        print(f"\n⚡ Running Benchmark: {name}...")

        # Python runs
        py_times, py_mems = [], []
        for _ in range(3):
            dur, mem, _ = run_and_measure(["python", t["py_file"]])
            py_times.append(dur)
            py_mems.append(mem)
        avg_py_time = sum(py_times) / len(py_times)
        max_py_mem = max(py_mems)

        # AETHER runs
        ae_times, ae_mems = [], []
        for _ in range(3):
            dur, mem, _ = run_and_measure(["aether", "run", t["ae_file"]])
            ae_times.append(dur)
            ae_mems.append(mem)
        avg_ae_time = sum(ae_times) / len(ae_times)
        max_ae_mem = max(ae_mems)

        # Winner determination
        if avg_ae_time < avg_py_time:
            ratio = avg_py_time / max(avg_ae_time, 0.0001)
            winner = f"AETHER ({ratio:.2f}x faster)"
        else:
            ratio = avg_ae_time / max(avg_py_time, 0.0001)
            winner = f"Python ({ratio:.2f}x faster)"

        results.append({
            "name": name,
            "py_time": avg_py_time,
            "py_mem": max_py_mem,
            "ae_time": avg_ae_time,
            "ae_mem": max_ae_mem,
            "winner": winner
        })

    print("\n" + "=" * 88)
    print("📊 FINAL HEAD-TO-HEAD SCORECARD TABLE (AETHER 2.0 vs PYTHON 3.14.4)")
    print("=" * 88)
    header = f"{'Benchmark Name':<35} | {'Python Time':<11} | {'AETHER Time':<11} | {'Winner':<22}"
    print(header)
    print("-" * 88)

    for r in results:
        py_str = f"{r['py_time']:.1f} ms"
        ae_str = f"{r['ae_time']:.1f} ms"
        print(f"{r['name']:<35} | {py_str:<11} | {ae_str:<11} | {r['winner']:<22}")

    print("=" * 88)
    print("\n💾 MEMORY CONSUMPTION FOOTPRINT (PEAK RSS WORKING SET):")
    print(f"{'Benchmark Name':<35} | {'Python Peak RAM':<15} | {'AETHER Peak RAM':<15}")
    print("-" * 75)
    for r in results:
        py_m_str = f"{r['py_mem']:.2f} MB"
        ae_m_str = f"{r['ae_mem']:.2f} MB"
        print(f"{r['name']:<35} | {py_m_str:<15} | {ae_m_str:<15}")
    print("=" * 88)

if __name__ == "__main__":
    benchmark_suite()
