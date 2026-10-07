# SPDX-License-Identifier: AGPL-3.0-or-later
"""A small animated GIF writer (no PIL inside Blender): one global 256-color palette chosen
from the frames, nearest-color mapping, LZW, infinite loop."""

import struct

import numpy as np


def _palette(frames):
    # Count colors at 5 bits per channel; keep the 256 most frequent bins.
    q = np.concatenate([(f >> 3).reshape(-1, 3).astype(np.int32) for f in frames])
    bins = (q[:, 0] << 10) | (q[:, 1] << 5) | q[:, 2]
    counts = np.bincount(bins, minlength=1 << 15)
    top = np.argsort(-counts, kind="stable")[:256]
    top = top[counts[top] > 0]
    pal = np.stack([(top >> 10) & 31, (top >> 5) & 31, top & 31], axis=1) * 8 + 4
    # Map every 15-bit bin to its nearest palette entry.
    allbins = np.arange(1 << 15)
    rgb = np.stack([(allbins >> 10) & 31, (allbins >> 5) & 31, allbins & 31], axis=1) * 8 + 4
    lut = np.empty(1 << 15, dtype=np.uint8)
    for s in range(0, 1 << 15, 4096):
        d = ((rgb[s:s + 4096, None, :] - pal[None, :, :]) ** 2).sum(axis=2)
        lut[s:s + 4096] = d.argmin(axis=1)
    full = np.zeros((256, 3), dtype=np.uint8)
    full[:len(pal)] = pal.clip(0, 255)
    return full, lut


def _lzw(indices, min_size=8):
    clear, eoi = 1 << min_size, (1 << min_size) + 1
    out = bytearray()
    acc = nbits = 0

    def emit(code, size):
        nonlocal acc, nbits
        acc |= code << nbits
        nbits += size
        while nbits >= 8:
            out.append(acc & 0xFF)
            acc >>= 8
            nbits -= 8

    size = min_size + 1
    table = {}
    nxt = eoi + 1
    emit(clear, size)
    data = indices.tobytes()
    prefix = data[0]
    for k in data[1:]:
        key = (prefix, k)
        code = table.get(key)
        if code is not None:
            prefix = code
            continue
        emit(prefix, size)
        if nxt < 4096:
            table[key] = nxt
            nxt += 1
            if nxt > (1 << size) and size < 12:
                size += 1
        else:
            emit(clear, size)
            table.clear()
            nxt = eoi + 1
            size = min_size + 1
        prefix = k
    emit(prefix, size)
    emit(eoi, size)
    if nbits:
        out.append(acc & 0xFF)
    blocks = bytearray()
    for i in range(0, len(out), 255):
        chunk = out[i:i + 255]
        blocks.append(len(chunk))
        blocks += chunk
    blocks.append(0)
    return bytes(blocks)


def write(path, frames, fps=30):
    """`frames`: equally sized uint8 RGB arrays (h, w, 3)."""
    h, w = frames[0].shape[:2]
    pal, lut = _palette(frames)
    out = bytearray(b"GIF89a")
    out += struct.pack("<HHBBB", w, h, 0xF7, 0, 0)
    out += pal.tobytes()
    out += b"\x21\xFF\x0BNETSCAPE2.0\x03\x01\x00\x00\x00"
    elapsed = 0
    for i, f in enumerate(frames):
        # GIF delays are in 1/100 s: alternate 3 and 4 to average the real frame rate.
        t = round((i + 1) * 100 / fps)
        delay, elapsed = t - elapsed, t
        q = (f >> 3).astype(np.int32)
        idx = lut[(q[..., 0] << 10) | (q[..., 1] << 5) | q[..., 2]].astype(np.uint8)
        out += b"\x21\xF9\x04\x00" + struct.pack("<H", delay) + b"\x00\x00"
        out += b"\x2C" + struct.pack("<HHHHB", 0, 0, w, h, 0)
        out += b"\x08" + _lzw(idx.ravel())
    out += b"\x3B"
    with open(path, "wb") as fh:
        fh.write(out)
