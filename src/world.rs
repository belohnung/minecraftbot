use derive_newtype::NewType;
use std::ops::Index;

#[repr(transparent)]
#[derive(NewType)]
pub struct ChunkBlockID(u8);

pub struct BiomeData {}
pub struct ChunkData {
    block_ids: [[[ChunkBlockID; 16]; 16]; 16],
}
impl ChunkData {
    pub fn new() -> ChunkData {
        ChunkData {
            block_ids: [[[0.into(); 16]; 16]; 16],
        }
    }
}

impl Index<(usize, usize, usize)> for ChunkData {
    type Output = ChunkBlockID;

    fn index(&self, index: (usize, usize, usize)) -> &Self::Output {
        unimplemented!()
    }
}

pub struct ChunkNibbleData {}
pub struct ChunkColumn {}

pub struct Chunk {}
pub struct World {}
