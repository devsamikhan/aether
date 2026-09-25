import os
import sys
import time
import subprocess

# Ensure UTF-8 output on Windows
if sys.platform == "win32":
    try:
        sys.stdout.reconfigure(encoding='utf-8', errors='replace')
    except Exception:
        pass

def measure_process(cmd, cwd=None):
    """Executes a command and returns elapsed_ms and stdout"""
    t0 = time.perf_counter()
    p = subprocess.Popen(
        cmd,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding='utf-8',
        errors='replace',
        cwd=cwd
    )
    stdout, stderr = p.communicate()
    t1 = time.perf_counter()
    elapsed_ms = (t1 - t0) * 1000.0
    return elapsed_ms, stdout.strip()

def main():
    print("=" * 90)
    print("⚡ AETHER 2.0 HEAVY STRESS BENCHMARK SHOOTOUT — DIRECT VS C++ & PYTHON")
    print("=" * 90)
    print("Platform: Windows 11 (8 CPU Cores)")
    print("Runtimes:")
    print("  • AETHER 2.0: target/release/aether.exe (Work-Stealing Green Fibers & SIMD-64 Engine)")
    print("  • C++:        MinGW GCC 6.3.0 (-O3 Full Optimization)")
    print("  • Python:     CPython 3.14.4 64-bit")
    print("-" * 90)

    base_dir = os.path.dirname(os.path.abspath(__file__))
    aether_exe = os.path.abspath(os.path.join(base_dir, "..", "..", "target", "release", "aether.exe"))

    # Compile C++ files
    print("🔨 Compiling C++ benchmark targets with g++ -O3...")
    subprocess.run(["g++", "-O3", os.path.join(base_dir, "concurrency_cpp.cpp"), "-o", os.path.join(base_dir, "concurrency_cpp.exe")], check=True)
    subprocess.run(["g++", "-O3", os.path.join(base_dir, "concurrency_1m_cpp.cpp"), "-o", os.path.join(base_dir, "concurrency_1m_cpp.exe")], check=True)
    subprocess.run(["g++", "-O3", os.path.join(base_dir, "compute_10m_cpp.cpp"), "-o", os.path.join(base_dir, "compute_10m_cpp.exe")], check=True)
    subprocess.run(["g++", "-O3", os.path.join(base_dir, "memory_churn_cpp.cpp"), "-o", os.path.join(base_dir, "memory_churn_cpp.exe")], check=True)
    print("✅ All C++ targets compiled successfully.\n")

    benchmarks = [
        {
            "name": "1. Concurrency Stream (500k Msgs)",
            "desc": "500,000 Messages Producer-Consumer Pipeline",
            "aether_cmd": [aether_exe, os.path.join(base_dir, "concurrency_aether.ae")],
            "cpp_cmd": [os.path.join(base_dir, "concurrency_cpp.exe")],
            "py_cmd": ["python", os.path.join(base_dir, "concurrency_python.py")],
        },
        {
            "name": "2. Concurrency Stream (1 Million Msgs)",
            "desc": "1,000,000 Messages Massive Green Fiber / Thread Ring",
            "aether_cmd": [aether_exe, os.path.join(base_dir, "concurrency_1m_aether.ae")],
            "cpp_cmd": [os.path.join(base_dir, "concurrency_1m_cpp.exe")],
            "py_cmd": ["python", os.path.join(base_dir, "test_1m_py.py")],
        },
        {
            "name": "3. Big Data Vector FMA (10M Floats)",
            "desc": "10 Million Floats Fused Multiply-Add + Parallel Sum",
            "aether_cmd": [aether_exe, os.path.join(base_dir, "test_big_compute.ae")],
            "cpp_cmd": [os.path.join(base_dir, "compute_10m_cpp.exe")],
            "py_cmd": ["python", os.path.join(base_dir, "compute_10m_python.py")],
        },
        {
            "name": "4. Memory Churn (500k Allocations)",
            "desc": "Rapid Heap Node Allocation & Immediate Deallocation",
            "aether_cmd": [aether_exe, os.path.join(base_dir, "memory_churn_aether.ae")],
            "cpp_cmd": [os.path.join(base_dir, "memory_churn_cpp.exe")],
            "py_cmd": ["python", os.path.join(base_dir, "memory_churn_python.py")],
        }
    ]

    results = []

    for b in benchmarks:
        print(f"▶ Benchmarking: {b['name']}...")
        ae_time, _ = measure_process(b["aether_cmd"])
        cpp_time, _ = measure_process(b["cpp_cmd"])
        py_time, _ = measure_process(b["py_cmd"])

        results.append({
            "name": b["name"],
            "aether": ae_time,
            "cpp": cpp_time,
            "py": py_time
        })
        print(f"   AETHER: {ae_time:.1f} ms | C++: {cpp_time:.1f} ms | Python: {py_time:.1f} ms")

    print("\n" + "=" * 90)
    print("🏆 FINAL HEAVY STRESS BENCHMARK SHOOTOUT RESULTS")
    print("=" * 90)
    print(f"{'Heavy Stress Workload':<38} | {'AETHER 2.0':<12} | {'C++ (-O3)':<12} | {'Python 3.14':<12} | {'Outcome'}")
    print("-" * 90)

    for r in results:
        ae = r["aether"]
        cpp = r["cpp"]
        py = r["py"]
        
        if ae < cpp and ae < py:
            winner = f"⚡ AETHER #1 ({cpp/ae:.1f}x vs C++)"
        elif ae < py:
            winner = f"🚀 AETHER {py/ae:.1f}x vs Py"
        else:
            winner = f"🎯 Near C++ speed"

        print(f"{r['name']:<38} | {ae:>8.1f} ms   | {cpp:>8.1f} ms   | {py:>8.1f} ms   | {winner}")

    print("=" * 90)

if __name__ == "__main__":
    main()
