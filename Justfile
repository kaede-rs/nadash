debugbuild:
  cargo build -vv

releaseuild:
  cargo build -rvv

usebuild:
  RUSTFLAGS="-C target-cpu=native" cargo build -rvv
