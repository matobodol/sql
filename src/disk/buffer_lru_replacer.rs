use std::collections::VecDeque;

pub type FrameId = usize;

pub struct LRUReplacer {
    entries: VecDeque<FrameId>,
}

impl LRUReplacer {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(capacity),
        }
    }

    pub fn victim(&mut self) -> Option<FrameId> {
        self.entries.pop_front()
    }

    pub fn pin(&mut self, frame_id: FrameId) {
        if let Some(pos) = self.entries.iter().position(|&id| id == frame_id) {
            self.entries.remove(pos);
        }
    }

    pub fn unpin(&mut self, frame_id: FrameId) {
        // Hapus syarat self.entries.len() < capacity agar tidak terjadi frame leak!
        if !self.entries.contains(&frame_id) {
            self.entries.push_back(frame_id);
        }
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }
}
