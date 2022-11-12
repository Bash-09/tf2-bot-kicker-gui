echo 0 | sudo tee /proc/sys/kernel/perf_event_paranoid
echo 0 | sudo tee /proc/sys/kernel/kptr_restrict
CARGO_PROFILE_RELEASE_DEBUG=true CARGO_PROFILE_RELEASE_STRIP=false cargo flamegraph
