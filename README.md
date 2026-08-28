# Nadash
Nadashは、Rustをベースに作られた「なでしこ」のコンパイラ・インタプリタです。
コンパイル時はなでしこをRustに変換し、インタプリタ使用時はLuaJITに変換します。

## Build
1. ターミナルを開く
2. `git clone https://github.com/tarutarudev/nadash.git && cd nadash && RUSTFLAGS="-C target-cpu=native" cargo build -r`を実行
