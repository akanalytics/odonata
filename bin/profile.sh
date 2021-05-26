#!/bin/bash
perf record -e task-clock,cycles,instructions,cache-references,cache-misses --call-graph dwarf -F 99 --output=./tmp/perf.data  ./target/release/odonata --perft 5
# chown andy.andy ./tmp/perf.data
perf report --input=./tmp/perf.data


