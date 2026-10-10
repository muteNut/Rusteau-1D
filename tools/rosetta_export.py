#!/usr/bin/env python3
"""Converts the neural network weights of the Rosetta pedotransfer functions to the binary file read by Rusteau.

Source: the Python package rosetta-soil (USDA-ARS U.S. Salinity Laboratory, https://github.com/usda-ars-ussl/rosetta-soil,
CC0 1.0), folder rosetta/data with the files rose1_*.npz and rose3_*.npz.

    python3 tools/rosetta_export.py <path to rosetta/data> crates/rusteau-core/data/rosetta.bin

Format (little endian): b"RSTA", u32 version (1), u32 number of models; per model: u8 name length, name, u32 ninput,
noutput, nboot, nlayer; per layer u32 nodes and u8 activation (0 sigmoid, 1 tansig, 2 linear, 3 relu); per input and
per output u8 scaler (0 asis, 1 division, 2 logtr, 3 affine, 4 minmax, 5 linear) and three f64 parameters; then for
every bootstrap member and layer the weights (nodes x inputs, row major) and the biases as f32.
"""
import struct
import sys
from pathlib import Path

import numpy as np

ACT = {"sigmoid": 0, "tansig": 1, "linear": 2, "relu": 3}
SCALE = {"asis": 0, "division": 1, "logtr": 2, "affine": 3, "minmax": 4, "linear": 5}
MODELS = [
    "rose1_mod2_0", "rose1_mod2_1", "rose1_mod3_0", "rose1_mod3_1", "rose1_mod4_0", "rose1_mod4_1",
    "rose1_mod5_0", "rose1_mod5_1", "rose1_unsat_k",
    "rose3_mod2_0", "rose3_mod3_0", "rose3_mod4_0", "rose3_mod5_0",
]


def scaler(c):
    p = [float(x) for x in np.atleast_1d(np.asarray(c["params"], dtype=float))]
    p = (p + [0.0, 0.0, 0.0])[:3]
    return struct.pack("<B3d", SCALE[c["scale"].lower()], *p)


def main(src, dst):
    out = bytearray(b"RSTA") + struct.pack("<II", 1, len(MODELS))
    for name in MODELS:
        d = np.load(Path(src) / f"{name}.npz", allow_pickle=True)
        ninput, noutput, nboot, nlayer = (int(d[k].item()) for k in ("ninput", "noutput", "nboot", "nlayer"))
        layers = d["layers"].tolist()
        nb = name.encode()
        out += struct.pack("<B", len(nb)) + nb + struct.pack("<IIII", ninput, noutput, nboot, nlayer)
        for l in layers:
            out += struct.pack("<IB", int(l["nnodes"]), ACT[l["activate"].lower()])
        for c in d["inputs"].tolist():
            out += scaler(c)
        for c in d["outputs"].tolist():
            out += scaler(c)
        weights, biases = d["weights"].tolist(), d["biases"].tolist()
        for b in range(nboot):
            nin = ninput
            for i, l in enumerate(layers):
                n = int(l["nnodes"])
                w = np.asarray(weights[b][i], dtype=np.float32).reshape(n, nin)
                bias = np.atleast_1d(np.asarray(biases[b][i], dtype=np.float32)).reshape(n)
                out += w.astype("<f4").tobytes() + bias.astype("<f4").tobytes()
                nin = n
    Path(dst).write_bytes(bytes(out))
    print(f"{dst}: {len(out)} bytes, {len(MODELS)} models")


if __name__ == "__main__":
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    main(sys.argv[1], sys.argv[2])
