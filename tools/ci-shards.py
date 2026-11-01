#!/usr/bin/env python3
# SPDX-License-Identifier: MPL-2.0
# SPDX-FileCopyrightText: 2026 Apibim
"""Balanced CI test shards from measured test durations.

    ci-shards.py plan --shards N --threads T --out DIR JUNIT.xml...
        Takes each test's mean duration over the reports it appears in, so the reports of
        several runs give a steadier plan than one run: hosted machines differ by about 10 %,
        and a heavy test by up to 12 s. Groups the tests: a binary with less than --split
        seconds of tests is one group, a larger one is split by suite module, and a test of at least
        --heavy seconds is a group of its own. It deals the groups to N shards, longest
        group first, each to the shard that would finish soonest when its tests start in list
        order on T threads, as nextest starts them. It writes the filtersets of shards
        1..N-1 to DIR/1.txt .. DIR/<N-1>.txt and prints each shard's predicted time.

    ci-shards.py filter --dir DIR --shard K --shards N
        Prints the nextest filterset of shard K. Shard N is everything the other shards'
        filtersets leave out, so a test added after the plan, or renamed, still runs there.
        Fails when DIR does not hold exactly N-1 shard files.

Nothing here can lose a test: shard N takes whatever the others do not, and CI's `ci-ok`
job (tools/ci-gate.py) checks on every run that the shards ran every listed test exactly once.

Standard library only.
"""

import argparse
import collections
import heapq
import os
import re
import sys
import xml.etree.ElementTree as ET

NAME = re.compile(r"^[A-Za-z0-9_:./-]+$")
MAX_FILTER = 16_000  # stay well under Windows' 32 K command line


def fail(message):
    print(f"error: {message}", file=sys.stderr)
    sys.exit(2)


def read_durations(paths):
    """{(binary_id, test): seconds}, the mean when a test appears in several reports."""
    samples = collections.defaultdict(list)
    for path in paths:
        try:
            root = ET.parse(path).getroot()
        except (OSError, ET.ParseError) as error:
            fail(f"cannot read the JUnit report {path}: {error}")
        for suite in root.iter("testsuite"):
            for case in suite.iter("testcase"):
                binary_id = case.get("classname") or suite.get("name")
                name = case.get("name")
                if not binary_id or not name:
                    fail(f"{path}: a testcase without a binary id or name")
                if not NAME.match(binary_id) or not NAME.match(name):
                    fail(f"{path}: unexpected characters in {binary_id} {name}")
                try:
                    seconds = float(case.get("time", "0"))
                except ValueError:
                    fail(f"{path}: {binary_id} {name} has no numeric time")
                samples[(binary_id, name)].append(seconds)
    if not samples:
        fail("the reports hold no tests")
    return {key: sum(values) / len(values) for key, values in samples.items()}


def group_key(binary_id, name, heavy, split):
    if heavy:
        return (binary_id, "=", name)
    if binary_id not in split:
        return (binary_id, "binary", "")
    module = name.split("::", 1)[0] if "::" in name else ""
    return (binary_id, "module", module)


def group_filter(key, heavy_in_module):
    binary_id, kind, value = key
    binary = f"binary_id(={binary_id})"
    if kind == "=":
        return f"({binary} and test(={value}))"
    if kind == "binary":
        if heavy_in_module:
            exclusions = " or ".join(f"test(={name})" for name in sorted(heavy_in_module))
            return f"({binary} and not ({exclusions}))"
        return binary
    selector = f"test(/^{value}::/)" if value else "not test(/::/)"
    expression = f"{binary} and {selector}"
    if heavy_in_module:
        exclusions = " or ".join(f"test(={name})" for name in sorted(heavy_in_module))
        expression += f" and not ({exclusions})"
    return f"({expression})"


def simulate(tests, threads):
    """Wall time of tests started in list order (binary id, then name) on `threads` threads."""
    slots = [0.0] * threads
    heapq.heapify(slots)
    end = 0.0
    for _, seconds in sorted(tests):
        start = heapq.heappop(slots)
        finish = start + seconds
        end = max(end, finish)
        heapq.heappush(slots, finish)
    return end


def plan(args):
    if args.shards < 2:
        fail("--shards must be at least 2")
    durations = read_durations(args.junit)
    totals = collections.Counter()
    for (binary_id, _), seconds in durations.items():
        totals[binary_id] += seconds
    # Only binaries with much test time are split by suite module; the rest stay whole, which
    # keeps the filtersets short.
    split = {binary_id for binary_id, total in totals.items() if total >= args.split}
    groups = collections.defaultdict(list)
    heavy_by_module = collections.defaultdict(set)
    for (binary_id, name), seconds in durations.items():
        heavy = seconds >= args.heavy
        key = group_key(binary_id, name, heavy, split)
        groups[key].append(((binary_id, name), seconds))
        if heavy:
            if binary_id in split:
                module = name.split("::", 1)[0] if "::" in name else ""
            else:
                module = ""
            heavy_by_module[(binary_id, module)].add(name)
    shards = [[] for _ in range(args.shards)]
    members = [[] for _ in range(args.shards)]
    order = sorted(groups, key=lambda k: (-sum(s for _, s in groups[k]), k))
    for key in order:
        best = min(
            range(args.shards),
            key=lambda i: (simulate(shards[i] + groups[key], args.threads), i),
        )
        shards[best].extend(groups[key])
        members[best].append(key)
    os.makedirs(args.out, exist_ok=True)
    for old in os.listdir(args.out):
        if old.endswith(".txt"):
            os.remove(os.path.join(args.out, old))
    for index in range(args.shards - 1):
        parts = []
        for key in sorted(members[index]):
            binary_id, kind, value = key
            heavy = (
                heavy_by_module.get((binary_id, value), set())
                if kind in ("module", "binary")
                else set()
            )
            parts.append(group_filter(key, heavy))
        text = "\n or ".join(parts) + "\n"
        with open(os.path.join(args.out, f"{index + 1}.txt"), "w", encoding="utf-8") as out:
            out.write(text)
    for index, tests in enumerate(shards):
        label = "rest" if index == args.shards - 1 else f"{index + 1}.txt"
        print(
            f"shard {index + 1} ({label}): {len(tests)} tests, "
            f"{sum(s for _, s in tests):.0f} s of tests, "
            f"predicted {simulate(tests, args.threads):.1f} s on {args.threads} threads"
        )
    complement = render(args.out, args.shards, args.shards)
    if len(complement) > MAX_FILTER:
        fail(f"the last shard's filter is {len(complement)} characters; use fewer groups")
    return 0


def render(directory, shard, shards):
    files = sorted(f for f in os.listdir(directory) if re.fullmatch(r"[0-9]+\.txt", f))
    expected = [f"{i}.txt" for i in range(1, shards)]
    if sorted(files, key=lambda f: int(f[:-4])) != expected:
        fail(f"{directory} holds {files}; {shards} shards need exactly {expected}")
    def read(index):
        with open(os.path.join(directory, f"{index}.txt"), encoding="utf-8") as handle:
            text = " ".join(handle.read().split())
        if not text:
            fail(f"{directory}/{index}.txt is empty")
        return text
    if not 1 <= shard <= shards:
        fail(f"shard {shard} is outside 1..{shards}")
    if shard < shards:
        return read(shard)
    return "not (" + " or ".join(f"({read(i)})" for i in range(1, shards)) + ")"


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n", 1)[0])
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("plan")
    p.add_argument("--shards", type=int, required=True)
    p.add_argument("--threads", type=int, required=True)
    p.add_argument("--heavy", type=float, default=8.0)
    p.add_argument("--split", type=float, default=60.0)
    p.add_argument("--out", required=True)
    p.add_argument("junit", nargs="+")
    f = sub.add_parser("filter")
    f.add_argument("--dir", required=True)
    f.add_argument("--shard", type=int, required=True)
    f.add_argument("--shards", type=int, required=True)
    args = parser.parse_args(argv)
    if args.command == "plan":
        return plan(args)
    print(render(args.dir, args.shard, args.shards))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
