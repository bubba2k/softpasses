#!/usr/bin/env python3

import subprocess
import time
import sys
import math

def perform_runs(num_runs, executable_path, branch_name):
    # Checkout the branch
    subprocess.run(["git", "checkout", branch_name], check=True)
    # Compile the branch
    subprocess.run(["cargo", "build", "--release"], check=True)

    run_times = []
    for run_idx in range(1, num_runs + 1):
        start = time.time()
        subprocess.run([executable_path], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        end = time.time()
        elapsed = end - start
        print(f"{branch_name}: {run_idx}/{num_runs}: {elapsed:.4f}s")
        run_times.append(elapsed)

    avg_time = sum(run_times) / num_runs
    variance = sum((t - avg_time) ** 2 for t in run_times) / num_runs
    stddev = variance ** 0.5
    if num_runs > 1:
        stderr = stddev / math.sqrt(num_runs)
        ci_low = avg_time - 1.96 * stderr
        ci_high = avg_time + 1.96 * stderr
        confidence_interval = (ci_low, ci_high)
    else:
        confidence_interval = None

    return {
        "num_runs": num_runs,
        "run_times": run_times,
        "average_time": avg_time,
        "variance": variance,
        "stddev": stddev,
        "confidence_interval": confidence_interval
    }

if __name__ == "__main__":
    if len(sys.argv) != 4:
        print(f"Usage: {sys.argv[0]} <branch1> <branch2> <num_runs>")
        sys.exit(1)
    BRANCH1 = sys.argv[1] if len(sys.argv) > 1 else ""
    BRANCH2 = sys.argv[2] if len(sys.argv) > 2 else ""
    NUM_RUNS = int(sys.argv[3])
    EXECUTABLE_PATH = "./target/release/weekend_of_rays"

    for branch in [BRANCH1, BRANCH2]:
        result = subprocess.run(["git", "rev-parse", "--verify", branch], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if result.returncode != 0:
            print(f"Error: Branch '{branch}' does not exist.")
            sys.exit(1)

    results1 = perform_runs(NUM_RUNS, EXECUTABLE_PATH, BRANCH1)
    results2 = perform_runs(NUM_RUNS, EXECUTABLE_PATH, BRANCH2)

    def print_stats(branch, results):
        print(f"\nResults for branch '{branch}':")
        print(f"  Runs: {results['num_runs']}")
        print(f"  Times: {results['run_times']}")
        print(f"  Average: {results['average_time']:.4f}s")
        print(f"  Stddev: {results['stddev']:.4f}s")
        if results['confidence_interval']:
            ci_low, ci_high = results['confidence_interval']
            print(f"  95% CI: [{ci_low:.4f}s, {ci_high:.4f}s]")

    print_stats(BRANCH1, results1)
    print_stats(BRANCH2, results2)