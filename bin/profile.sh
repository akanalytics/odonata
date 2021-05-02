#!/bin/bash
perf record --call-graph dwarf -f --output=./tmp/perf.data  ./target/release/profile
chown andy.andy ./tmp/perf.data
perf report --input=./tmp/perf.data

