//! The glTF binary subset a pack may carry (11 §3): meshes with positions, vertex colors and
//! one skin, materials by name, TRS nodes and animations. No images, textures, samplers,
//! cameras, extensions or external URIs. Every offset is bounds-checked before it is read.

use crate::json::Json;

/// Container cap (11 §4): checked before anything is parsed.
pub const MAX_GLB_BYTES: usize = 4 * 1024 * 1024;
const MAX_ELEMENTS: usize = 1 << 20;

const ALLOWED_TOP: &[&str] = &[
    "asset",
    "scene",
    "scenes",
    "nodes",
    "meshes",
    "skins",
    "animations",
    "materials",
    "accessors",
    "bufferViews",
    "buffers",
    "extras",
];
const ALLOWED_ATTRIBUTES: &[&str] = &["POSITION", "COLOR_0", "JOINTS_0", "WEIGHTS_0"];

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub parent: Option<usize>,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub mesh: Option<usize>,
    pub skin: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Primitive {
    pub material: Option<usize>,
    pub triangles: usize,
    pub has_color: bool,
    pub positions: Vec<[f32; 3]>,
    /// COLOR_0 as RGBA (alpha 1 when the attribute is RGB); empty without one.
    pub colors: Vec<[f32; 4]>,
    /// Triangle list indices (0..n when the primitive isn't indexed).
    pub indices: Vec<u32>,
    pub joints: Vec<[u32; 4]>,
    pub weights: Vec<[f32; 4]>,
}

#[derive(Clone, Debug)]
pub struct Channel {
    pub node: usize,
    /// "translation", "rotation" or "scale".
    pub path: String,
    pub step: bool,
    pub times: Vec<f32>,
    /// One value of 3 (translation, scale) or 4 (rotation, xyzw) components per time.
    pub values: Vec<Vec<f32>>,
}

#[derive(Clone, Debug)]
pub struct Animation {
    pub name: String,
    pub channels: Vec<Channel>,
}

#[derive(Clone, Debug, Default)]
pub struct Model {
    pub nodes: Vec<Node>,
    /// Joint node indices of each skin.
    pub skins: Vec<Vec<usize>>,
    pub meshes: Vec<Vec<Primitive>>,
    pub materials: Vec<String>,
    pub animations: Vec<Animation>,
}

/// A node's world transform: translation, rotation (xyzw), scale.
type Trs = ([f32; 3], [f32; 4], [f32; 3]);

impl Model {
    pub fn triangles(&self) -> usize {
        self.meshes.iter().flatten().map(|p| p.triangles).sum()
    }

    pub fn node_by_name(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.name == name)
    }

    /// Rest-pose world positions of every node (glTF space: +Y up, the model faces +Z).
    pub fn world_positions(&self) -> Vec<[f32; 3]> {
        let mut world: Vec<Option<Trs>> = vec![None; self.nodes.len()];
        fn solve(m: &Model, i: usize, world: &mut Vec<Option<Trs>>) -> Trs {
            if let Some(w) = world[i] {
                return w;
            }
            let n = &m.nodes[i];
            let w = match n.parent {
                None => (n.translation, n.rotation, n.scale),
                Some(p) => {
                    let (pt, pr, ps) = solve(m, p, world);
                    let local = [n.translation[0] * ps[0], n.translation[1] * ps[1], n.translation[2] * ps[2]];
                    let r = rotate(pr, local);
                    (
                        [pt[0] + r[0], pt[1] + r[1], pt[2] + r[2]],
                        qmul(pr, n.rotation),
                        [ps[0] * n.scale[0], ps[1] * n.scale[1], ps[2] * n.scale[2]],
                    )
                }
            };
            world[i] = Some(w);
            w
        }
        (0..self.nodes.len()).map(|i| solve(self, i, &mut world).0).collect()
    }
}

pub fn qmul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

pub fn rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let p = qmul(qmul(q, [v[0], v[1], v[2], 0.0]), [-q[0], -q[1], -q[2], q[3]]);
    [p[0], p[1], p[2]]
}

/// Angle between two unit quaternions, in degrees.
pub fn quat_angle_deg(a: &[f32], b: &[f32]) -> f32 {
    let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]).abs().min(1.0);
    (2.0 * dot.acos()).to_degrees()
}

/// Splits a GLB container and parses its JSON chunk.
pub fn read_container(bytes: &[u8]) -> Result<(Json, &[u8]), String> {
    if bytes.len() > MAX_GLB_BYTES {
        return Err(format!("{} bytes, over the {} byte cap", bytes.len(), MAX_GLB_BYTES));
    }
    let u32_at = |o: usize| -> Result<u32, String> {
        bytes
            .get(o..o + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .ok_or_else(|| "truncated GLB".to_string())
    };
    if u32_at(0)? != 0x4654_6C67 || u32_at(4)? != 2 {
        return Err("not a glTF 2.0 binary".into());
    }
    if u32_at(8)? as usize != bytes.len() {
        return Err("GLB length field does not match the file size".into());
    }
    let jlen = u32_at(12)? as usize;
    if u32_at(16)? != 0x4E4F_534A || !jlen.is_multiple_of(4) {
        return Err("first chunk must be 4-byte aligned JSON".into());
    }
    let json_bytes = bytes.get(20..20 + jlen).ok_or("JSON chunk past the end of the file")?;
    let json = Json::parse(json_bytes).map_err(|e| e.to_string())?;
    let mut off = 20 + jlen;
    let mut bin: &[u8] = &[];
    if off < bytes.len() {
        let blen = u32_at(off)? as usize;
        if u32_at(off + 4)? != 0x004E_4942 || !blen.is_multiple_of(4) {
            return Err("second chunk must be 4-byte aligned BIN".into());
        }
        bin = bytes.get(off + 8..off + 8 + blen).ok_or("BIN chunk past the end of the file")?;
        off += 8 + blen;
    }
    if off != bytes.len() {
        return Err("unexpected chunks after BIN".into());
    }
    Ok((json, bin))
}

/// Reads the strict subset into a [`Model`].
pub fn parse(bytes: &[u8]) -> Result<Model, String> {
    let (doc, bin) = read_container(bytes)?;
    for key in doc.keys() {
        match key {
            "images" | "textures" | "samplers" => return Err(format!("`{key}`: packs carry no images or textures")),
            "cameras" => return Err("`cameras`: not allowed in packs".into()),
            "extensions" | "extensionsUsed" | "extensionsRequired" => {
                return Err(format!("`{key}`: glTF extensions are not allowed"));
            }
            k if !ALLOWED_TOP.contains(&k) => return Err(format!("unknown top-level key `{k}`")),
            _ => {}
        }
    }
    let version = doc.get("asset").and_then(|a| a.get("version")).and_then(Json::as_str);
    if version != Some("2.0") {
        return Err("asset.version must be \"2.0\"".into());
    }
    let buffers = doc.get("buffers").map(Json::as_arr).unwrap_or(&[]);
    if buffers.len() > 1 {
        return Err("only one buffer (the GLB BIN chunk) is allowed".into());
    }
    if let Some(b) = buffers.first() {
        if b.get("uri").is_some() {
            return Err("buffers may not reference external URIs".into());
        }
        let len = b.get("byteLength").and_then(Json::as_index).ok_or("buffer without byteLength")?;
        if len > bin.len() {
            return Err("buffer is longer than the BIN chunk".into());
        }
    }
    let r = Reader { doc: &doc, bin };

    let mut m = Model::default();
    let jnodes = doc.get("nodes").map(Json::as_arr).unwrap_or(&[]);
    for (i, n) in jnodes.iter().enumerate() {
        if n.get("matrix").is_some() {
            return Err(format!("node {i}: use translation/rotation/scale, not matrix"));
        }
        m.nodes.push(Node {
            name: n.get("name").and_then(Json::as_str).unwrap_or("").to_string(),
            parent: None,
            translation: floats(n.get("translation"), [0.0; 3])?,
            rotation: floats(n.get("rotation"), [0.0, 0.0, 0.0, 1.0])?,
            scale: floats(n.get("scale"), [1.0; 3])?,
            mesh: opt_index(n.get("mesh"))?,
            skin: opt_index(n.get("skin"))?,
        });
    }
    for (i, n) in jnodes.iter().enumerate() {
        for c in n.get("children").map(Json::as_arr).unwrap_or(&[]) {
            let c = c.as_index().filter(|&c| c < m.nodes.len()).ok_or(format!("node {i}: bad child"))?;
            if m.nodes[c].parent.is_some() || c == i {
                return Err(format!("node {c} has more than one parent"));
            }
            m.nodes[c].parent = Some(i);
        }
    }
    for i in 0..m.nodes.len() {
        let (mut cur, mut steps) = (i, 0);
        while let Some(p) = m.nodes[cur].parent {
            cur = p;
            steps += 1;
            if steps > m.nodes.len() {
                return Err("node hierarchy has a cycle".into());
            }
        }
    }

    for mat in doc.get("materials").map(Json::as_arr).unwrap_or(&[]) {
        if mat.keys().any(|k| k.ends_with("Texture"))
            || mat.get("pbrMetallicRoughness").is_some_and(|p| p.keys().any(|k| k.ends_with("Texture")))
        {
            return Err("materials may not reference textures".into());
        }
        m.materials.push(mat.get("name").and_then(Json::as_str).unwrap_or("").to_string());
    }

    for skin in doc.get("skins").map(Json::as_arr).unwrap_or(&[]) {
        let joints = skin.get("joints").map(Json::as_arr).unwrap_or(&[]);
        let joints: Option<Vec<usize>> = joints.iter().map(|j| j.as_index().filter(|&j| j < m.nodes.len())).collect();
        m.skins.push(joints.ok_or("skin joint out of range")?);
    }

    for (mi, mesh) in doc.get("meshes").map(Json::as_arr).unwrap_or(&[]).iter().enumerate() {
        let mut prims = Vec::new();
        for p in mesh.get("primitives").map(Json::as_arr).unwrap_or(&[]) {
            let mode = p.get("mode").and_then(Json::as_index).unwrap_or(4);
            if mode != 4 {
                return Err(format!("mesh {mi}: only triangle lists are allowed"));
            }
            if p.get("targets").is_some() {
                return Err(format!("mesh {mi}: morph targets are not allowed"));
            }
            let attrs = p.get("attributes").ok_or(format!("mesh {mi}: primitive without attributes"))?;
            for k in attrs.keys() {
                if !ALLOWED_ATTRIBUTES.contains(&k) {
                    return Err(format!("mesh {mi}: attribute `{k}` is not allowed"));
                }
            }
            let attr = |k: &str| attrs.get(k).and_then(Json::as_index);
            let positions = r.vec3(attr("POSITION").ok_or(format!("mesh {mi}: no POSITION"))?)?;
            let count = positions.len();
            let joints = match attr("JOINTS_0") {
                Some(a) => r.raw4(a)?.into_iter().map(|v| v.map(|x| x as u32)).collect(),
                None => Vec::new(),
            };
            let weights = match attr("WEIGHTS_0") {
                Some(a) => r.normalized4(a)?,
                None => Vec::new(),
            };
            let colors: Vec<[f32; 4]> = match attr("COLOR_0") {
                Some(a) => {
                    let (n, flat) = r.read(a)?;
                    let width = flat.len().checked_div(n).unwrap_or(4);
                    if n != count || !(width == 3 || width == 4) {
                        return Err(format!("mesh {mi}: COLOR_0 must be RGB or RGBA per vertex"));
                    }
                    flat.chunks(width)
                        .map(|c| [c[0] as f32, c[1] as f32, c[2] as f32, c.get(3).map_or(1.0, |a| *a as f32)])
                        .collect()
                }
                None => Vec::new(),
            };
            let has_color = !colors.is_empty();
            if (!joints.is_empty() && joints.len() != count) || (!weights.is_empty() && weights.len() != count) {
                return Err(format!("mesh {mi}: attribute counts differ"));
            }
            let indices: Vec<u32> = match p.get("indices").and_then(Json::as_index) {
                Some(a) => {
                    let (_, idx) = r.read(a)?;
                    if idx.iter().any(|&i| i < 0.0 || i as usize >= count) {
                        return Err(format!("mesh {mi}: index out of range"));
                    }
                    idx.iter().map(|&i| i as u32).collect()
                }
                None => (0..count as u32).collect(),
            };
            let triangles = indices.len() / 3;
            prims.push(Primitive {
                material: opt_index(p.get("material"))?,
                triangles,
                has_color,
                positions,
                colors,
                indices,
                joints,
                weights,
            });
        }
        m.meshes.push(prims);
    }

    for a in doc.get("animations").map(Json::as_arr).unwrap_or(&[]) {
        let samplers = a.get("samplers").map(Json::as_arr).unwrap_or(&[]);
        let mut channels = Vec::new();
        for c in a.get("channels").map(Json::as_arr).unwrap_or(&[]) {
            let s = c.get("sampler").and_then(Json::as_index).and_then(|s| samplers.get(s)).ok_or("bad sampler")?;
            let t = c.get("target").ok_or("channel without target")?;
            let node = t
                .get("node")
                .and_then(Json::as_index)
                .filter(|&n| n < m.nodes.len())
                .ok_or("channel without a node")?;
            let path = t.get("path").and_then(Json::as_str).unwrap_or("").to_string();
            let width = match path.as_str() {
                "translation" | "scale" => 3,
                "rotation" => 4,
                _ => return Err(format!("animation channel path `{path}` is not allowed")),
            };
            let interp = s.get("interpolation").and_then(Json::as_str).unwrap_or("LINEAR");
            if interp != "LINEAR" && interp != "STEP" {
                return Err(format!("interpolation `{interp}` is not allowed"));
            }
            let (_, times) = r.read(s.get("input").and_then(Json::as_index).ok_or("sampler without input")?)?;
            let (n, flat) = r.read(s.get("output").and_then(Json::as_index).ok_or("sampler without output")?)?;
            if n != times.len() || flat.len() != n * width || times.windows(2).any(|w| w[1] <= w[0]) {
                return Err("animation sampler times and values disagree".into());
            }
            channels.push(Channel {
                node,
                path,
                step: interp == "STEP",
                times: times.iter().map(|&t| t as f32).collect(),
                values: flat.chunks(width).map(|c| c.iter().map(|&v| v as f32).collect()).collect(),
            });
        }
        m.animations.push(Animation { name: a.get("name").and_then(Json::as_str).unwrap_or("").to_string(), channels });
    }
    Ok(m)
}

fn floats<const N: usize>(v: Option<&Json>, default: [f32; N]) -> Result<[f32; N], String> {
    let Some(v) = v else { return Ok(default) };
    let a = v.as_arr();
    if a.len() != N {
        return Err("bad transform".into());
    }
    let mut out = default;
    for (o, x) in out.iter_mut().zip(a) {
        *o = x.as_f64().ok_or("bad transform")? as f32;
    }
    Ok(out)
}

fn opt_index(v: Option<&Json>) -> Result<Option<usize>, String> {
    match v {
        None => Ok(None),
        Some(j) => j.as_index().map(Some).ok_or_else(|| "bad index".to_string()),
    }
}

struct Reader<'a> {
    doc: &'a Json,
    bin: &'a [u8],
}

impl Reader<'_> {
    /// Element count and every component as f64 (normalized integers scaled to 0..1).
    fn read(&self, accessor: usize) -> Result<(usize, Vec<f64>), String> {
        let acc =
            self.doc.get("accessors").map(Json::as_arr).and_then(|a| a.get(accessor)).ok_or("accessor out of range")?;
        if acc.get("sparse").is_some() {
            return Err("sparse accessors are not allowed".into());
        }
        let count = acc.get("count").and_then(Json::as_index).ok_or("accessor without count")?;
        if count > MAX_ELEMENTS {
            return Err("accessor too large".into());
        }
        let width = match acc.get("type").and_then(Json::as_str) {
            Some("SCALAR") => 1,
            Some("VEC2") => 2,
            Some("VEC3") => 3,
            Some("VEC4") => 4,
            Some("MAT4") => 16,
            _ => return Err("unsupported accessor type".into()),
        };
        let ct = acc.get("componentType").and_then(Json::as_index).ok_or("accessor without componentType")?;
        let size = match ct {
            5120 | 5121 => 1,
            5122 | 5123 => 2,
            5125 | 5126 => 4,
            _ => return Err("unsupported component type".into()),
        };
        let normalized = matches!(acc.get("normalized"), Some(Json::Bool(true)));
        let view_i = acc.get("bufferView").and_then(Json::as_index).ok_or("accessor without bufferView")?;
        let view = self
            .doc
            .get("bufferViews")
            .map(Json::as_arr)
            .and_then(|v| v.get(view_i))
            .ok_or("bufferView out of range")?;
        let v_off = view.get("byteOffset").and_then(Json::as_index).unwrap_or(0);
        let v_len = view.get("byteLength").and_then(Json::as_index).ok_or("bufferView without byteLength")?;
        if v_off.checked_add(v_len).is_none_or(|end| end > self.bin.len()) {
            return Err("bufferView outside the BIN chunk".into());
        }
        let elem = size * width;
        let stride = view.get("byteStride").and_then(Json::as_index).unwrap_or(elem);
        if stride < elem {
            return Err("byteStride smaller than an element".into());
        }
        let a_off = acc.get("byteOffset").and_then(Json::as_index).unwrap_or(0);
        if count > 0 && a_off + stride * (count - 1) + elem > v_len {
            return Err("accessor outside its bufferView".into());
        }
        let data = &self.bin[v_off..v_off + v_len];
        let mut out = Vec::with_capacity(count * width);
        for e in 0..count {
            for c in 0..width {
                let o = a_off + e * stride + c * size;
                let b = &data[o..o + size];
                let v = match ct {
                    5126 => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
                    5125 => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
                    5123 => {
                        let x = u16::from_le_bytes([b[0], b[1]]) as f64;
                        if normalized { x / 65535.0 } else { x }
                    }
                    5122 => {
                        let x = i16::from_le_bytes([b[0], b[1]]) as f64;
                        if normalized { (x / 32767.0).max(-1.0) } else { x }
                    }
                    5121 => {
                        let x = b[0] as f64;
                        if normalized { x / 255.0 } else { x }
                    }
                    _ => {
                        let x = b[0] as i8 as f64;
                        if normalized { (x / 127.0).max(-1.0) } else { x }
                    }
                };
                if !v.is_finite() {
                    return Err("non-finite value in accessor".into());
                }
                out.push(v);
            }
        }
        Ok((count, out))
    }

    fn vec3(&self, a: usize) -> Result<Vec<[f32; 3]>, String> {
        let (n, v) = self.read(a)?;
        if v.len() != n * 3 {
            return Err("expected VEC3".into());
        }
        Ok(v.chunks(3).map(|c| [c[0] as f32, c[1] as f32, c[2] as f32]).collect())
    }

    fn raw4(&self, a: usize) -> Result<Vec<[f64; 4]>, String> {
        let (n, v) = self.read(a)?;
        if v.len() != n * 4 {
            return Err("expected VEC4".into());
        }
        Ok(v.chunks(4).map(|c| [c[0], c[1], c[2], c[3]]).collect())
    }

    fn normalized4(&self, a: usize) -> Result<Vec<[f32; 4]>, String> {
        Ok(self.raw4(a)?.into_iter().map(|c| c.map(|x| x as f32)).collect())
    }
}
