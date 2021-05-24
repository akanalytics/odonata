#!/bin/bash
perf record --call-graph dwarf -F 99 --output=./tmp/perf.data  ./target/release/odonata --perft 6
# chown andy.andy ./tmp/perf.data
perf report --input=./tmp/perf.data


