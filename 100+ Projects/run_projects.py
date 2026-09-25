import subprocess
import time
import os
import sys
from pathlib import Path

if hasattr(sys.stdout, 'reconfigure'):
    sys.stdout.reconfigure(encoding='utf-8', errors='replace')

def run_project(file_path):
    t0 = time.perf_counter()
    res = subprocess.run(
        ["aether", "run", str(file_path)],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace"
    )
    t1 = time.perf_counter()
    duration_ms = (t1 - t0) * 1000.0
    passed = (res.returncode == 0)
    return passed, duration_ms, res.stdout, res.stderr


def main():
    base_dir = Path(__file__).parent.resolve()
    print("=" * 86)
    print("🚀 AETHER 100+ REAL-WORLD PROJECTS SUITE — MASTER AUTOMATED TEST RUNNER")
    print("=" * 86)
    print(f"Workspace: {base_dir}")
    print("-" * 86)

    # Find all runnable .ae files recursively
    ae_files = sorted(list(base_dir.rglob("*.ae")))
    if not ae_files:
        print("No .ae files found to execute!")
        return

    print(f"Discovered {len(ae_files)} runnable AETHER 2.0 applications.\n")

    results = []
    passed_count = 0
    failed_count = 0
    total_time_ms = 0.0

    header = f"{'Domain':<26} | {'Project Application':<34} | {'Status':<8} | {'Latency':<9}"
    print(header)
    print("-" * 86)

    for f in ae_files:
        domain_name = f.parent.name
        project_name = f.stem
        
        passed, duration_ms, stdout, stderr = run_project(f)
        total_time_ms += duration_ms

        if passed:
            passed_count += 1
            status_str = "PASS ✅"
        else:
            failed_count += 1
            status_str = "FAIL ❌"

        lat_str = f"{duration_ms:.1f} ms"
        print(f"{domain_name:<26} | {project_name:<34} | {status_str:<8} | {lat_str:<9}")
        
        if not passed:
            print(f"    🚨 Error Output:\n{stderr or stdout}\n")

        results.append({
            "domain": domain_name,
            "project": project_name,
            "passed": passed,
            "duration_ms": duration_ms
        })

    print("-" * 86)
    print("📊 EXECUTION SUMMARY REPORT:")
    print(f"  • Total Executed Projects:  {len(ae_files)}")
    print(f"  • Succeeded:                {passed_count} ✅")
    print(f"  • Failed:                   {failed_count} ❌")
    print(f"  • Overall Success Rate:     {(passed_count / len(ae_files)) * 100.0:.1f}%")
    print(f"  • Total Suite Runtime:      {total_time_ms / 1000.0:.2f} seconds")
    print("=" * 86)

if __name__ == "__main__":
    main()
