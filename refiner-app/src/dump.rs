use aurora_refiner_types::aurora_block::AuroraBlock;
use std::path::PathBuf;

struct Storage {
    path: PathBuf,
}

impl Storage {
    fn get_path(&self, block_height: u64) -> PathBuf {
        // TODO: Create folders if required
        todo!()
    }

    pub fn dump(&self, block: AuroraBlock) -> std::io::Result<()> {
        let path = self.get_path(block.height);
        // TODO: Serialize as json and store it.
        Ok(())
    }
}
