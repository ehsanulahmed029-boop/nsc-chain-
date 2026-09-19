use crate::block::Block;

#[derive(Debug, Clone)]
pub struct BlockMessage {
    pub sender: String,
    pub block: Block,
}

impl BlockMessage {
    pub fn new(
        sender: String,
        block: Block,
    ) -> Self {
        Self {
            sender,
            block,
        }
    }
}
