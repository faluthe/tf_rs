//! Poses a model from an animation in its files, so it can be drawn without an entity behind it.
//! Reads the model files as the model cache loaded them, studiohdr_t and what it points to.

use crate::types::Matrix3x4;

/// Byte offsets of the studiohdr_t fields read
mod studiohdr {
    pub const ID: usize = 0;
    pub const LENGTH: usize = 76;
    /// Count and offset of the mstudiobone_t table
    pub const BONES: usize = 156;
    /// Count and offset of the mstudioanimdesc_t table
    pub const LOCAL_ANIMS: usize = 180;
    /// Count and offset of the mstudioseqdesc_t table
    pub const LOCAL_SEQS: usize = 188;
    /// Enough to hold every field above
    pub const MIN_SIZE: usize = 196;
}

mod mstudiobone {
    pub const SIZE: usize = 216;
    pub const NAME: usize = 0;
    pub const PARENT: usize = 4;
    pub const POS: usize = 32;
    pub const QUAT: usize = 44;
    pub const ROT: usize = 60;
    pub const POS_SCALE: usize = 72;
    pub const ROT_SCALE: usize = 84;
}

mod mstudioseqdesc {
    pub const SIZE: usize = 212;
    pub const LABEL: usize = 4;
    pub const NUM_BLENDS: usize = 56;
    /// Offset of the i16 animation indices of the blends
    pub const ANIM_INDICES: usize = 60;
}

mod mstudioanimdesc {
    pub const SIZE: usize = 100;
    pub const FPS: usize = 8;
    pub const FRAMES: usize = 16;
    pub const ANIM_BLOCK: usize = 52;
    pub const ANIM_INDEX: usize = 56;
    pub const SECTION_INDEX: usize = 80;
    pub const SECTION_FRAMES: usize = 84;
}

mod mstudioanimsections {
    pub const SIZE: usize = 8;
    pub const ANIM_BLOCK: usize = 0;
    pub const ANIM_INDEX: usize = 4;
}

/// The header of one bone's tracks
mod mstudioanim {
    pub const BONE: usize = 0;
    pub const FLAGS: usize = 1;
    pub const NEXT: usize = 2;
    pub const DATA: usize = 4;

    pub const RAWPOS: u8 = 0x01;
    pub const RAWROT: u8 = 0x02;
    pub const ANIMPOS: u8 = 0x04;
    pub const ANIMROT: u8 = 0x08;
    pub const DELTA: u8 = 0x10;
    pub const RAWROT2: u8 = 0x20;
}

/// A loaded model file, every read checked against its length
pub struct Studio<'a> {
    b: &'a [u8],
}

/// Local position and rotation of a bone, relative to its parent
#[derive(Clone, Copy)]
struct Local {
    pos: [f32; 3],
    quat: [f32; 4],
}

struct Bone {
    parent: i32,
    local: Local,
    rot: [f32; 3],
    pos_scale: [f32; 3],
    rot_scale: [f32; 3],
}

/// An animation of a model
pub struct Anim {
    offset: usize,
    pub fps: f32,
    pub frames: i32,
    /// Where the bone tracks start relative to `offset`, or the sections when split into them
    anim_index: i32,
    section_index: i32,
    section_frames: i32,
    anim_block: i32,
}

impl<'a> Studio<'a> {
    /// The studiohdr_t at `ptr`, none if it isn't one
    ///
    /// # Safety
    /// `ptr` must be null or point to a loaded model that stays loaded while this is used
    pub unsafe fn from_ptr(ptr: *const u8) -> Option<Self> {
        if ptr.is_null() {
            return None;
        }
        let head = Studio {
            b: unsafe { std::slice::from_raw_parts(ptr, studiohdr::MIN_SIZE) },
        };
        if head.bytes(studiohdr::ID)? != *b"IDST" {
            return None;
        }
        let length = usize::try_from(head.i32(studiohdr::LENGTH)?).ok()?;
        if length < studiohdr::MIN_SIZE {
            return None;
        }
        let b = unsafe { std::slice::from_raw_parts(ptr, length) };
        Some(Studio { b })
    }

    pub fn ptr(&self) -> *const u8 {
        self.b.as_ptr()
    }

    fn bytes<const N: usize>(&self, at: usize) -> Option<[u8; N]> {
        self.b.get(at..at.checked_add(N)?)?.try_into().ok()
    }

    fn u8(&self, at: usize) -> Option<u8> {
        self.b.get(at).copied()
    }

    fn i32(&self, at: usize) -> Option<i32> {
        self.bytes(at).map(i32::from_le_bytes)
    }

    fn i16(&self, at: usize) -> Option<i16> {
        self.bytes(at).map(i16::from_le_bytes)
    }

    fn u16(&self, at: usize) -> Option<u16> {
        self.bytes(at).map(u16::from_le_bytes)
    }

    fn f32(&self, at: usize) -> Option<f32> {
        self.bytes(at).map(f32::from_le_bytes)
    }

    fn f32s<const N: usize>(&self, at: usize) -> Option<[f32; N]> {
        let mut out = [0.0; N];
        for (i, v) in out.iter_mut().enumerate() {
            *v = self.f32(at + i * 4)?;
        }
        Some(out)
    }

    fn str(&self, at: usize) -> Option<&'a str> {
        let rest = self.b.get(at..)?;
        let end = rest.iter().position(|&c| c == 0)?;
        std::str::from_utf8(&rest[..end]).ok()
    }

    /// `base` plus the relative offset stored at `base + field`
    fn rel(&self, base: usize, field: usize) -> Option<usize> {
        base.checked_add_signed(self.i32(base + field)? as isize)
    }

    /// (count, offset) pair of the header at `at`
    fn table(&self, at: usize, size: usize) -> Option<impl Iterator<Item = usize> + use<>> {
        let count = self.i32(at)?.max(0) as usize;
        let start = self.i32(at + 4)?.max(0) as usize;
        start.checked_add(count.checked_mul(size)?)?;
        Some((0..count).map(move |i| start + i * size))
    }

    fn bones(&self) -> Option<Vec<Bone>> {
        use mstudiobone as b;
        self.table(studiohdr::BONES, b::SIZE)?
            .map(|o| {
                Some(Bone {
                    parent: self.i32(o + b::PARENT)?,
                    local: Local {
                        pos: self.f32s(o + b::POS)?,
                        quat: self.f32s(o + b::QUAT)?,
                    },
                    rot: self.f32s(o + b::ROT)?,
                    pos_scale: self.f32s(o + b::POS_SCALE)?,
                    rot_scale: self.f32s(o + b::ROT_SCALE)?,
                })
            })
            .collect()
    }

    /// Names of the bones, in the order of `bones`
    fn bone_names(&self) -> Option<Vec<&'a str>> {
        self.table(studiohdr::BONES, mstudiobone::SIZE)?
            .map(|o| self.str(self.rel(o, mstudiobone::NAME)?))
            .collect()
    }

    /// The first animation of the sequence called `name`, ignoring case
    pub fn sequence_anim(&self, name: &str) -> Option<Anim> {
        use mstudioseqdesc as s;
        let seq = self.table(studiohdr::LOCAL_SEQS, s::SIZE)?.find(|&o| {
            self.rel(o, s::LABEL)
                .and_then(|at| self.str(at))
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
        })?;
        if self.i32(seq + s::NUM_BLENDS)? < 1 {
            return None;
        }
        let index = self.i16(self.rel(seq, s::ANIM_INDICES)?)?;
        self.anim(index as usize)
    }

    fn anim(&self, index: usize) -> Option<Anim> {
        use mstudioanimdesc as a;
        let offset = self.table(studiohdr::LOCAL_ANIMS, a::SIZE)?.nth(index)?;
        Some(Anim {
            offset,
            fps: self.f32(offset + a::FPS)?,
            frames: self.i32(offset + a::FRAMES)?,
            anim_block: self.i32(offset + a::ANIM_BLOCK)?,
            anim_index: self.i32(offset + a::ANIM_INDEX)?,
            section_index: self.i32(offset + a::SECTION_INDEX)?,
            section_frames: self.i32(offset + a::SECTION_FRAMES)?,
        })
    }

    /// The pose of the model's first animation, or the bones' own values without one, which are
    /// unposed
    fn reference(&self, bones: &[Bone]) -> Vec<Local> {
        self.anim(0)
            .and_then(|a| self.sample(bones, &a, 0))
            .unwrap_or_else(|| bones.iter().map(|b| b.local).collect())
    }

    /// Where the bone tracks of `anim` at `frame` start, and the frame within them
    fn tracks(&self, anim: &Anim, frame: i32) -> Option<(usize, i32)> {
        let (block, index, frame) = if anim.section_frames > 0 {
            use mstudioanimsections as s;
            // The last frame is kept on its own after the sections
            let (section, frame) = if anim.frames > anim.section_frames && frame == anim.frames - 1
            {
                (anim.frames / anim.section_frames + 1, 0)
            } else {
                let section = frame / anim.section_frames;
                (section, frame - section * anim.section_frames)
            };
            let at = anim.offset.checked_add_signed(
                anim.section_index as isize + section as isize * s::SIZE as isize,
            )?;
            (
                self.i32(at + s::ANIM_BLOCK)?,
                self.i32(at + s::ANIM_INDEX)?,
                frame,
            )
        } else {
            (anim.anim_block, anim.anim_index, frame)
        };
        // Data in separate animation blocks isn't loaded with the model
        if block != 0 {
            return None;
        }
        Some((anim.offset.checked_add_signed(index as isize)?, frame))
    }

    /// Value of a compressed track at `frame`
    fn track_value(&self, mut at: usize, frame: i32, scale: f32) -> Option<f32> {
        let mut k = frame;
        loop {
            let valid = self.u8(at)? as i32;
            let total = self.u8(at + 1)? as i32;
            if total == 0 {
                return Some(0.0);
            }
            if total > k {
                let i = if valid > k { k + 1 } else { valid };
                return Some(self.i16(at + i as usize * 2)? as f32 * scale);
            }
            k -= total;
            at += (valid as usize + 1) * 2;
        }
    }

    /// Three tracks whose offsets from `at` are stored there, zero where a track is missing
    fn track_values(&self, at: usize, frame: i32, scale: [f32; 3]) -> Option<[f32; 3]> {
        let mut out = [0.0; 3];
        for j in 0..3 {
            let offset = self.i16(at + j * 2)?;
            if offset > 0 {
                out[j] = self.track_value(at + offset as usize, frame, scale[j])?;
            }
        }
        Some(out)
    }

    /// Local transforms of every bone at `frame` of `anim`, bones it doesn't move keep their own
    fn sample(&self, bones: &[Bone], anim: &Anim, frame: i32) -> Option<Vec<Local>> {
        use mstudioanim as m;
        let mut locals: Vec<Local> = bones.iter().map(|b| b.local).collect();
        let (mut at, frame) = self.tracks(anim, frame)?;
        loop {
            let index = self.u8(at + m::BONE)? as usize;
            let flags = self.u8(at + m::FLAGS)?;
            let next = self.i16(at + m::NEXT)?;
            let bone = bones.get(index)?;
            let data = at + m::DATA;
            let has = |flag: u8| flags & flag != 0;
            let delta = has(m::DELTA);

            let quat = if has(m::RAWROT) {
                quat48(self.u16(data)?, self.u16(data + 2)?, self.u16(data + 4)?)
            } else if has(m::RAWROT2) {
                quat64(u64::from_le_bytes(self.bytes(data)?))
            } else if has(m::ANIMROT) {
                let mut angles = self.track_values(data, frame, bone.rot_scale)?;
                if !delta {
                    for (a, r) in angles.iter_mut().zip(bone.rot) {
                        *a += r;
                    }
                }
                angle_quat(angles)
            } else if delta {
                [0.0, 0.0, 0.0, 1.0]
            } else {
                bone.local.quat
            };

            let pos = if has(m::RAWPOS) {
                let at = data
                    + if has(m::RAWROT) { 6 } else { 0 }
                    + if has(m::RAWROT2) { 8 } else { 0 };
                [
                    half(self.u16(at)?),
                    half(self.u16(at + 2)?),
                    half(self.u16(at + 4)?),
                ]
            } else if has(m::ANIMPOS) {
                let at = data + if has(m::ANIMROT) { 6 } else { 0 };
                let mut pos = self.track_values(at, frame, bone.pos_scale)?;
                if !delta {
                    for (p, b) in pos.iter_mut().zip(bone.local.pos) {
                        *p += b;
                    }
                }
                pos
            } else if delta {
                [0.0; 3]
            } else {
                bone.local.pos
            };

            locals[index] = Local { pos, quat };
            if next <= 0 {
                return Some(locals);
            }
            at += next as usize;
        }
    }

    /// Local transforms at `time` seconds into looping `anim`, between its two nearest frames
    fn sample_at(&self, bones: &[Bone], anim: &Anim, time: f32) -> Option<Vec<Local>> {
        if anim.frames <= 1 || anim.fps <= 0.0 {
            return self.sample(bones, anim, 0);
        }
        let last = (anim.frames - 1) as f32;
        let frame = (time * anim.fps).rem_euclid(last);
        let first = frame.floor() as i32;
        let mut a = self.sample(bones, anim, first)?;
        let b = self.sample(bones, anim, (first + 1).min(anim.frames - 1))?;
        let s = frame.fract();
        for (a, b) in a.iter_mut().zip(&b) {
            *a = blend(a, b, s);
        }
        Some(a)
    }
}

/// A model posed by an animation in another file, with everything that doesn't change between
/// frames read once. Only holds offsets into the files, so it stays right when the cache moves them
pub struct Rig {
    anim: Anim,
    model_bones: Vec<Bone>,
    anim_bones: Vec<Bone>,
    /// For each model bone, the animation bone of the same name
    animated: Vec<Option<usize>>,
    /// Pose of the model bones the animation lacks
    reference: Vec<Local>,
}

impl Rig {
    /// `model` posed by `anim` of `anims`, matching bones by name. None if they can't be read
    pub fn new(model: &Studio, anims: &Studio, anim: Anim) -> Option<Self> {
        let model_bones = model.bones()?;
        let anim_bones = anims.bones()?;
        let anim_names = anims.bone_names()?;
        let animated = model
            .bone_names()?
            .iter()
            .map(|name| anim_names.iter().position(|n| n == name))
            .collect();
        Some(Rig {
            reference: model.reference(&model_bones),
            anim,
            model_bones,
            anim_bones,
            animated,
        })
    }

    /// Bone to model matrices `time` seconds into the animation, `anims` being the file it was
    /// made from
    pub fn pose(&self, anims: &Studio, time: f32) -> Option<Vec<Matrix3x4>> {
        let animated = anims.sample_at(&self.anim_bones, &self.anim, time)?;
        let locals: Vec<Local> = self
            .animated
            .iter()
            .zip(&self.reference)
            .map(|(i, reference)| i.map_or(*reference, |i| animated[i]))
            .collect();
        Some(chain(&self.model_bones, &locals, |_| None))
    }
}

/// A model attached to another: bones sharing a name follow the parent's, the rest hang off them
/// in the attachment's reference pose
pub struct Merge {
    bones: Vec<Bone>,
    reference: Vec<Local>,
    /// For each bone, the parent model's bone of the same name
    parent_bone: Vec<Option<usize>>,
}

impl Merge {
    pub fn new(attachment: &Studio, parent: &Studio) -> Option<Self> {
        let bones = attachment.bones()?;
        let parent_names = parent.bone_names()?;
        let parent_bone = attachment
            .bone_names()?
            .iter()
            .map(|name| parent_names.iter().position(|n| n == name))
            .collect();
        Some(Merge {
            reference: attachment.reference(&bones),
            bones,
            parent_bone,
        })
    }

    /// Bone to model matrices with the parent posed by `parent_bones`
    pub fn pose(&self, parent_bones: &[Matrix3x4]) -> Vec<Matrix3x4> {
        chain(&self.bones, &self.reference, |i| {
            self.parent_bone[i].and_then(|p| parent_bones.get(p).copied())
        })
    }
}

/// Bone to model matrices from local transforms, `fixed` overriding any bone
fn chain(
    bones: &[Bone],
    locals: &[Local],
    fixed: impl Fn(usize) -> Option<Matrix3x4>,
) -> Vec<Matrix3x4> {
    let mut out: Vec<Matrix3x4> = Vec::with_capacity(bones.len());
    for (i, (bone, local)) in bones.iter().zip(locals).enumerate() {
        let matrix = fixed(i).unwrap_or_else(|| {
            let own = matrix(local);
            match usize::try_from(bone.parent).ok().and_then(|p| out.get(p)) {
                Some(parent) => concat(parent, &own),
                None => own,
            }
        });
        out.push(matrix);
    }
    out
}

fn quat48(x: u16, y: u16, zw: u16) -> [f32; 4] {
    let x = (x as f32 - 32768.0) / 32768.0;
    let y = (y as f32 - 32768.0) / 32768.0;
    let z = ((zw & 0x7FFF) as f32 - 16384.0) / 16384.0;
    let w = (1.0 - x * x - y * y - z * z).max(0.0).sqrt();
    [x, y, z, if zw & 0x8000 != 0 { -w } else { w }]
}

fn quat64(v: u64) -> [f32; 4] {
    let part = |shift: u32| ((v >> shift) & 0x1F_FFFF) as f64;
    let x = ((part(0) - 1048576.0) / 1048576.5) as f32;
    let y = ((part(21) - 1048576.0) / 1048576.5) as f32;
    let z = ((part(42) - 1048576.0) / 1048576.5) as f32;
    let w = (1.0 - x * x - y * y - z * z).max(0.0).sqrt();
    [x, y, z, if v >> 63 != 0 { -w } else { w }]
}

/// IEEE half precision float
fn half(h: u16) -> f32 {
    let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = ((h >> 10) & 0x1F) as i32;
    let mant = (h & 0x3FF) as f32;
    sign * match exp {
        0 => mant * 2f32.powi(-24),
        31 => f32::INFINITY,
        _ => (1.0 + mant / 1024.0) * 2f32.powi(exp - 15),
    }
}

/// Quaternion of radian euler angles, as the engine's AngleQuaternion
fn angle_quat(a: [f32; 3]) -> [f32; 4] {
    let (sy, cy) = (a[2] * 0.5).sin_cos();
    let (sp, cp) = (a[1] * 0.5).sin_cos();
    let (sr, cr) = (a[0] * 0.5).sin_cos();
    let (sr_cp, cr_sp) = (sr * cp, cr * sp);
    let (cr_cp, sr_sp) = (cr * cp, sr * sp);
    [
        sr_cp * cy - cr_sp * sy,
        cr_sp * cy + sr_cp * sy,
        cr_cp * sy - sr_sp * cy,
        cr_cp * cy + sr_sp * sy,
    ]
}

/// `s` of the way from `a` to `b`
fn blend(a: &Local, b: &Local, s: f32) -> Local {
    let dot: f32 = a.quat.iter().zip(b.quat).map(|(x, y)| x * y).sum();
    // Take the short way round
    let sign = if dot < 0.0 { -1.0 } else { 1.0 };
    let quat: [f32; 4] = std::array::from_fn(|i| a.quat[i] * (1.0 - s) + b.quat[i] * s * sign);
    let len = quat
        .iter()
        .map(|v| v * v)
        .sum::<f32>()
        .sqrt()
        .max(f32::EPSILON);
    let pos = std::array::from_fn(|i| a.pos[i] * (1.0 - s) + b.pos[i] * s);
    Local {
        pos,
        quat: quat.map(|v| v / len),
    }
}

fn matrix(local: &Local) -> Matrix3x4 {
    let [x, y, z, w] = local.quat;
    let p = local.pos;
    Matrix3x4 {
        m: [
            [
                1.0 - 2.0 * (y * y + z * z),
                2.0 * (x * y - w * z),
                2.0 * (x * z + w * y),
                p[0],
            ],
            [
                2.0 * (x * y + w * z),
                1.0 - 2.0 * (x * x + z * z),
                2.0 * (y * z - w * x),
                p[1],
            ],
            [
                2.0 * (x * z - w * y),
                2.0 * (y * z + w * x),
                1.0 - 2.0 * (x * x + y * y),
                p[2],
            ],
        ],
    }
}

fn concat(a: &Matrix3x4, b: &Matrix3x4) -> Matrix3x4 {
    let mut out = Matrix3x4::default();
    for i in 0..3 {
        for j in 0..4 {
            let mut v: f32 = (0..3).map(|k| a.m[i][k] * b.m[k][j]).sum();
            if j == 3 {
                v += a.m[i][3];
            }
            out.m[i][j] = v;
        }
    }
    out
}
