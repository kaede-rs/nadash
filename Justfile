build: debug:
  cargo build -vv

build: release:
  cargo build -rvv

build: use
  RUSTFLAGS="-C target-cpu=native" cargo build -rvv
