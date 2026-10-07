# SPDX-License-Identifier: AGPL-3.0-or-later
"""GLB post-processing: key reduction (10 §8.3) and a canonical binary layout.

The glTF exporter samples every animation channel on every frame. Linear channels whose keys
lie within tolerance of the interpolation between their neighbours are dropped (rotations by
angle, translations and scales by distance), and channels that never leave the bone's rest
value are removed: **a missing channel means "at rest"** for the runtime. The binary chunk is
then rebuilt with one tightly packed, 4-byte aligned buffer view per distinct accessor (identical
accessors are shared), so the output depends only on the content: the same .blend gives the
same bytes.
"""

import json
import struct

import numpy as np

ROT_TOL_DEG = 0.1
POS_TOL_M = 0.0005
SCALE_TOL = 0.001

_COMPONENT = {5120: np.int8, 5121: np.uint8, 5122: np.int16, 5123: np.uint16, 5125: np.uint32, 5126: np.float32}
_WIDTH = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT2": 4, "MAT3": 9, "MAT4": 16}


def read(path):
    with open(path, "rb") as f:
        data = f.read()
    magic, version, length = struct.unpack_from("<III", data, 0)
    assert magic == 0x46546C67 and version == 2 and length == len(data), "not a glTF 2 binary"
    jlen, jtype = struct.unpack_from("<II", data, 12)
    assert jtype == 0x4E4F534A
    doc = json.loads(data[20:20 + jlen].decode("utf-8"))
    off = 20 + jlen
    bin_chunk = b""
    if off < len(data):
        blen, btype = struct.unpack_from("<II", data, off)
        assert btype == 0x004E4942
        bin_chunk = data[off + 8:off + 8 + blen]
    return doc, bin_chunk


def _accessor_array(doc, bin_chunk, index):
    acc = doc["accessors"][index]
    view = doc["bufferViews"][acc["bufferView"]]
    dtype = np.dtype(_COMPONENT[acc["componentType"]]).newbyteorder("<")
    width = _WIDTH[acc["type"]]
    start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
    stride = view.get("byteStride", 0)
    if stride and stride != dtype.itemsize * width:
        rows = [np.frombuffer(bin_chunk, dtype, width, start + i * stride) for i in range(acc["count"])]
        return np.array(rows)
    return np.frombuffer(bin_chunk, dtype, acc["count"] * width, start).reshape(acc["count"], width)


def _reduce(times, values, path):
    """Greedy key reduction for one LINEAR channel. Returns the kept indices."""
    n = len(times)
    if n <= 2:
        return list(range(n))

    def ok(i, j):
        t = (times[i + 1:j] - times[i]) / (times[j] - times[i])
        a, b = values[i], values[j]
        if path == "rotation":
            if np.dot(a, b) < 0:
                b = -b
            interp = a[None, :] * (1 - t[:, None]) + b[None, :] * t[:, None]
            interp /= np.linalg.norm(interp, axis=1)[:, None]
            dots = np.abs(np.sum(interp * values[i + 1:j], axis=1)).clip(0, 1)
            return np.all(np.degrees(2 * np.arccos(dots)) <= ROT_TOL_DEG)
        interp = a[None, :] * (1 - t[:, None]) + b[None, :] * t[:, None]
        tol = POS_TOL_M if path == "translation" else SCALE_TOL
        return np.all(np.linalg.norm(interp - values[i + 1:j], axis=1) <= tol)

    kept = [0]
    i = 0
    while i < n - 1:
        j = i + 2
        while j < n and ok(i, j):
            j += 1
        kept.append(j - 1)
        i = j - 1
    # Constant channels collapse to their two end keys (or one if equal).
    return kept


def process(path):
    """Reduce animation keys and rewrite `path` with the canonical layout. Returns stats."""
    doc, bin_chunk = read(path)
    arrays = [np.array(_accessor_array(doc, bin_chunk, i)) for i in range(len(doc.get("accessors", [])))]
    before = after = dropped = 0
    nodes = doc.get("nodes", [])
    for anim in doc.get("animations", []):
        channels = []
        for ch in anim["channels"]:
            sampler = anim["samplers"][ch["sampler"]]
            times = arrays[sampler["input"]][:, 0].astype(np.float64)
            values = arrays[sampler["output"]].astype(np.float64)
            target = ch["target"]["path"]
            if sampler.get("interpolation", "LINEAR") != "LINEAR":
                # The exporter writes constant channels as two-key STEP samplers.
                if _at_rest(values, nodes[ch["target"]["node"]], target):
                    dropped += 1
                else:
                    channels.append(ch)
                continue
            keep = _reduce(times, values, target)
            before += len(times)
            if len(keep) <= 2 and _at_rest(values[keep], nodes[ch["target"]["node"]], target):
                dropped += 1
                continue
            channels.append(ch)
            after += len(keep)
            # Every channel gets its own input/output accessors after reduction.
            arrays.append(times[keep].astype(np.float32).reshape(-1, 1))
            doc["accessors"].append({"componentType": 5126, "count": len(keep), "type": "SCALAR",
                                     "min": [float(times[keep][0])], "max": [float(times[keep][-1])]})
            sampler["input"] = len(arrays) - 1
            out_acc = dict(doc["accessors"][sampler["output"]])
            out_acc.pop("bufferView", None)
            out_acc.pop("byteOffset", None)
            out_acc.pop("min", None)
            out_acc.pop("max", None)
            out_acc["count"] = len(keep)
            arrays.append(values[keep].astype(np.float32))
            doc["accessors"].append(out_acc)
            sampler["output"] = len(arrays) - 1
        # Keep only the samplers the remaining channels use, renumbered in order.
        used = sorted({c["sampler"] for c in channels})
        renum = {old: new for new, old in enumerate(used)}
        anim["samplers"] = [anim["samplers"][i] for i in used]
        for c in channels:
            c["sampler"] = renum[c["sampler"]]
        anim["channels"] = channels
    _rewrite(path, doc, arrays)
    return {"keys_before": before, "keys_after": after, "channels_at_rest_dropped": dropped}


def _at_rest(values, node, path):
    if path == "rotation":
        rest = np.array(node.get("rotation", [0, 0, 0, 1]), dtype=np.float64)
        dots = np.abs(values @ rest).clip(0, 1)
        return bool(np.all(np.degrees(2 * np.arccos(dots)) <= ROT_TOL_DEG))
    if path == "translation":
        rest = np.array(node.get("translation", [0, 0, 0]), dtype=np.float64)
        return bool(np.all(np.linalg.norm(values - rest, axis=1) <= POS_TOL_M))
    if path == "scale":
        rest = np.array(node.get("scale", [1, 1, 1]), dtype=np.float64)
        return bool(np.all(np.linalg.norm(values - rest, axis=1) <= SCALE_TOL))
    return False


def _rewrite(path, doc, arrays):
    # Drop accessors nothing references any more, then lay out one view per accessor.
    used = set()
    for mesh in doc.get("meshes", []):
        for prim in mesh["primitives"]:
            used.update(prim["attributes"].values())
            if "indices" in prim:
                used.add(prim["indices"])
    for skin in doc.get("skins", []):
        if "inverseBindMatrices" in skin:
            used.add(skin["inverseBindMatrices"])
    for anim in doc.get("animations", []):
        for s in anim["samplers"]:
            used.update((s["input"], s["output"]))
    remap = {}
    seen = {}

    blob = bytearray()
    views, accessors = [], []
    for old in sorted(used):
        acc = dict(doc["accessors"][old])
        dtype = np.dtype(_COMPONENT[acc["componentType"]]).newbyteorder("<")
        data = np.ascontiguousarray(arrays[old], dtype=dtype).tobytes()
        key = (acc["componentType"], acc["type"], acc.get("normalized", False), data)
        if key in seen:
            remap[old] = seen[key]
            continue
        seen[key] = remap[old] = len(accessors)
        while len(blob) % 4:
            blob.append(0)
        view = {"buffer": 0, "byteOffset": len(blob), "byteLength": len(data)}
        old_view = doc["bufferViews"][acc["bufferView"]] if "bufferView" in acc else {}
        if "target" in old_view:
            view["target"] = old_view["target"]
        blob += data
        acc.pop("byteOffset", None)
        acc["bufferView"] = len(views)
        views.append(view)
        accessors.append(acc)
    while len(blob) % 4:
        blob.append(0)

    for mesh in doc.get("meshes", []):
        for prim in mesh["primitives"]:
            prim["attributes"] = {k: remap[v] for k, v in prim["attributes"].items()}
            if "indices" in prim:
                prim["indices"] = remap[prim["indices"]]
    for skin in doc.get("skins", []):
        if "inverseBindMatrices" in skin:
            skin["inverseBindMatrices"] = remap[skin["inverseBindMatrices"]]
    for anim in doc.get("animations", []):
        for s in anim["samplers"]:
            s["input"], s["output"] = remap[s["input"]], remap[s["output"]]
    doc["accessors"] = accessors
    doc["bufferViews"] = views
    doc["buffers"] = [{"byteLength": len(blob)}]

    js = json.dumps(doc, separators=(",", ":"), ensure_ascii=True).encode("utf-8")
    js += b" " * ((4 - len(js) % 4) % 4)
    out = struct.pack("<III", 0x46546C67, 2, 12 + 8 + len(js) + 8 + len(blob))
    out += struct.pack("<II", len(js), 0x4E4F534A) + js
    out += struct.pack("<II", len(blob), 0x004E4942) + bytes(blob)
    with open(path, "wb") as f:
        f.write(out)
