use crate::vm::value::ValueTag;

pub const MONOMORPHIC_THRESHOLD: u64 = 3;
pub const MEGAMORPHIC_THRESHOLD: u64 = 5;
pub const MEGAMORPHIC_DEOPT_THRESHOLD: u64 = 10;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum InlineCacheKind {
    #[default]
    Polymorphic,
    Monomorphic {
        target: usize,
        tag: ValueTag,
    },
    Megamorphic,
}

#[derive(Debug, Clone)]
pub struct InlineCache {
    pub kind: InlineCacheKind,
    pub cached_target: Option<usize>,
    pub cached_type: Option<ValueTag>,
    pub hit_count: u64,
    pub miss_count: u64,
    pub deopt_count: u64,
}

impl Default for InlineCache {
    fn default() -> Self {
        Self::new()
    }
}

impl InlineCache {
    pub fn new() -> Self {
        InlineCache {
            kind: InlineCacheKind::Polymorphic,
            cached_target: None,
            cached_type: None,
            hit_count: 0,
            miss_count: 0,
            deopt_count: 0,
        }
    }

    pub fn update(&mut self, target: usize, tag: ValueTag) {
        match &mut self.kind {
            InlineCacheKind::Polymorphic => {
                self.cached_target = Some(target);
                self.cached_type = Some(tag);
                self.hit_count += 1;
                if self.hit_count >= MONOMORPHIC_THRESHOLD {
                    self.kind = InlineCacheKind::Monomorphic { target, tag };
                }
            }
            InlineCacheKind::Monomorphic { target: t, tag: tg } => {
                if *t == target && *tg == tag {
                    self.hit_count += 1;
                } else {
                    self.kind = InlineCacheKind::Megamorphic;
                    self.cached_target = None;
                    self.cached_type = None;
                    self.miss_count += 1;
                    self.deopt_count += 1;
                }
            }
            InlineCacheKind::Megamorphic => {
                self.miss_count += 1;
            }
        }
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hit_count + self.miss_count;
        if total == 0 {
            0.0
        } else {
            self.hit_count as f64 / total as f64
        }
    }

    pub fn is_monomorphic(&self) -> bool {
        matches!(self.kind, InlineCacheKind::Monomorphic { .. })
    }

    pub fn is_megamorphic(&self) -> bool {
        matches!(self.kind, InlineCacheKind::Megamorphic)
    }

    pub fn should_deopt(&self) -> bool {
        self.deopt_count >= MEGAMORPHIC_DEOPT_THRESHOLD
    }

    pub fn invalidate(&mut self) {
        self.kind = InlineCacheKind::Polymorphic;
        self.cached_target = None;
        self.cached_type = None;
        self.hit_count = 0;
        self.miss_count = 0;
        self.deopt_count = 0;
    }
}

#[derive(Debug, Default)]
pub struct CodeCache {
    pub baseline_count: usize,
    pub optimized_count: usize,
}

#[derive(Debug, Clone)]
pub struct CallCounter {
    pub func_index: usize,
    pub count: u64,
}

impl CallCounter {
    pub fn new(func_index: usize) -> Self {
        CallCounter {
            func_index,
            count: 0,
        }
    }
}
