# Benchmark
A quick-and-dirty benchmark of Servente, using [plow](https://github.com/six-ddc/plow).
Note that these results are not representative of real-world performance, and
are not to be used for any kind of comparison.

The benchmark was run on Manjaro Linux (kernel 5.10.161-1-MANJARO), AMD Ryzen 5 2400G
and 32 GB of RAM.

## Reproducing
To reproduce these results, run the following commands:
```bash
git clone https://github.com/usadson/servente.git
cd servente
cargo build --release
mkdir wwwroot
echo "Hello, world!" > wwwroot/test.txt
cargo run --release
```

In another terminal, run the following commands:
```bash
go install github.com/six-ddc/plow@latest
~/go/bin/plow -k https://localhost:8080/test.txt -c500 -n 1000000 -d 30s
```

## Results
```
Summary:
  Elapsed         11s
  Count        632126
    2xx        632126
  RPS       57126.502
  Reads    18.663MB/s
  Writes    4.815MB/s

Statistics    Min      Mean    StdDev      Max
  Latency    104µs    8.712ms  6.838ms  221.079ms
  RPS       49433.38  57126.6  2622.1   58809.26
```

The results indicate that at the time of writing, Servente can handle 50000 to
58000 requests per second, with an average latency of 8.7 milliseconds.
