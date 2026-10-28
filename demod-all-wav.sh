#!/bin/zsh
for f in ./data/field/nov26-hw/*.wav; do
    # do some stuff here with "$f"
    # remember to quote it or spaces may misbehave
    cargo run --release --bin demod_wav_hifiberry $f
done