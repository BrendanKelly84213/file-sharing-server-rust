#[allow(dead_code)]
pub struct FNode {
    block_index: i16, // index of the data block; negative if the block is not in use
    next: i16,        // index of the next fnode in the file; -1 if there is none
}

#[allow(dead_code)]
impl FNode {
    pub fn new(block_index: i16) -> Self {
        Self {
            block_index,
            next: -1,
        }
    }

    // Getters and Setters
    pub fn get_block_index(&self) -> i16 {
        self.block_index
    }

    pub fn set_block_index(&mut self, block_index: i16) {
        self.block_index = block_index;
    }

    pub fn get_next(&self) -> i16 {
        self.next
    }

    pub fn set_next(&mut self, next: i16) {
        self.next = next;
    }
}
