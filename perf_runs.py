#!/usr/bin/env python3

import subprocess
import time
import sys
import math

def perform_runs(num_runs, executable_path):
    print(f"Performing {num_runs} runs.")
    run_times = []
    for run_idx in range(1, num_runs + 1):
        start = time.time()
        subprocess.run([executable_path], check=True)
        end = time.time()
        elapsed = end - start
        run_times.append(elapsed)
        print(f"Run {run_idx}/{num_runs} took {elapsed:.4f} seconds.")
        print("Done.\n")

        avg_time = sum(run_times) / num_runs
        variance = sum((t - avg_time) ** 2 for t in run_times) / num_runs
        stddev = variance ** 0.5
        print(f"Average time: {avg_time:.4f} seconds")
        print(f"Variance: {variance:.4f}")
        print(f"Standard deviation: {stddev:.4f}")
        # Calculate 95% confidence interval for the mean
        if num_runs > 1:
            stderr = stddev / math.sqrt(num_runs)
            ci_low = avg_time - 1.96 * stderr
            ci_high = avg_time + 1.96 * stderr
            print(f"95% confidence interval for mean: [{ci_low:.4f}, {ci_high:.4f}] seconds")
        else:
            print("Not enough runs to calculate confidence interval.")


if __name__ == "__main__":
    NUM_RUNS = int(sys.argv[1]) if len(sys.argv) > 1 else 5
    EXECUTABLE_PATH = "./target/release/weekend_of_rays"
    perform_runs(NUM_RUNS, EXECUTABLE_PATH)