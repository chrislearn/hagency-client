"""Summarize the isolated native soak; never equate logical history with stored rows."""
import argparse
import json
from datetime import datetime
from pathlib import Path


def rows(path):
    return [json.loads(line) for line in path.read_text(encoding="utf-8-sig").splitlines() if line.strip()]


def slope(points):
    if len(points) < 2:
        return None
    xs, ys = zip(*points)
    mx, my = sum(xs) / len(xs), sum(ys) / len(ys)
    denominator = sum((x - mx) ** 2 for x in xs)
    return sum((x - mx) * (y - my) for x, y in points) / denominator if denominator else 0


def summarize(directory):
    samples = rows(directory / "samples.jsonl")
    processes = rows(directory / "process.jsonl")
    completion = json.loads((directory / "completion.json").read_text(encoding="utf-8-sig"))
    warm = [sample for sample in samples if sample["elapsed_s"] >= 300]
    live = [(sample["elapsed_s"] / 60, sample["rust_allocated"] - sample["rust_deallocated"])
            for sample in warm if sample.get("rust_allocated")]
    summary = {
        "completion": completion,
        "samples": len(samples),
        "seconds_sampled": samples[-1]["elapsed_s"],
        "ticks": samples[-1]["ticks"],
        "contexts": samples[-1]["contexts"],
        "history_modes": sorted({sample["logical_history"] for sample in samples}),
        "max_texture_bytes": max(sample["texture_bytes"] for sample in samples),
        "max_image_working_bytes": max(sample["image_working_bytes"] for sample in samples),
        "max_instance_bytes": max(sample["instance_bytes"] for sample in samples),
        "max_refusals": max(sample["refusals"] for sample in samples),
        "max_items": max(sample["state"]["items"] or 0 for sample in samples),
        "max_widgets": max(sample["state"]["widgets"] for sample in samples),
        "max_timer_p99_ms": max(sample["timer_p99_ms"] for sample in warm or samples),
        "max_private_bytes": max(process["private_bytes"] for process in processes),
        "final_private_bytes": processes[-1]["private_bytes"],
        "max_threads": max(process["threads"] for process in processes),
        "warm_rust_live_slope_bytes_per_minute": slope(live),
    }
    for field in ("tree_nodes", "tree_paths", "heap_bytes", "recording_bytes", "font_layout_bytes"):
        values = [sample[field] for sample in samples if field in sample]
        if values:
            summary["max_" + field] = max(values)
            summary["final_" + field] = values[-1]
    for field in ("gpu_dedicated_bytes", "gpu_shared_bytes"):
        values = [process[field] for process in processes if process.get(field) is not None]
        if values:
            summary["max_" + field] = max(values)
            summary["final_" + field] = values[-1]
    first_time = datetime.fromisoformat(processes[0]["utc"]).timestamp()
    for field in ("private_bytes", "gpu_dedicated_bytes", "gpu_shared_bytes"):
        points = [((datetime.fromisoformat(process["utc"]).timestamp() - first_time) / 60, process[field])
                  for process in processes if process.get(field) is not None
                  and datetime.fromisoformat(process["utc"]).timestamp() - first_time >= 300]
        summary["warm_" + field + "_slope_per_minute"] = slope(points)
    errors = []
    if completion["exit_code"] != 0 or completion.get("elapsed_seconds", 0) < completion["requested_seconds"]:
        errors.append("run did not complete its requested duration")
    if summary["seconds_sampled"] < completion["requested_seconds"] - 15:
        errors.append("native sampling stopped early")
    if summary["contexts"] < completion["requested_seconds"] // 10:
        errors.append("insufficient SDK context replacements")
    if summary["max_refusals"] or summary["max_items"] > 205 or summary["max_widgets"] > 64:
        errors.append("timeline/renderer bounds exceeded")
    if summary["max_texture_bytes"] > 256 * 1024**2 or summary["max_instance_bytes"] > 32 * 1024**2:
        errors.append("GPU admission budget exceeded")
    if summary.get("max_heap_bytes", 0) > 128 * 1024**2 or summary.get("max_recording_bytes", 0) > 32 * 1024**2:
        errors.append("owned CPU working-set budget exceeded")
    if summary.get("max_tree_nodes", 0) > 20000 or summary.get("max_tree_paths", 0) > 50000:
        errors.append("widget metadata grew beyond the fixture envelope")
    if live and slope(live) > 1024**2:
        errors.append("Rust retained allocations still grow above 1 MiB/min after warmup")
    for field, allowance in (("private_bytes", 5 * 1024**2), ("gpu_dedicated_bytes", 1024**2), ("gpu_shared_bytes", 1024**2)):
        growth = summary["warm_" + field + "_slope_per_minute"]
        if growth is not None and growth > allowance:
            errors.append(field + " continues to grow after warmup")
    summary["acceptance_errors"] = errors
    return summary


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    args = parser.parse_args()
    result = summarize(args.directory)
    (args.directory / "summary.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(result, indent=2))
    raise SystemExit(bool(result["acceptance_errors"]))
