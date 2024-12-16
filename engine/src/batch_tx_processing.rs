use aurora_engine_sdk::io::IO;
use engine_standalone_storage::{
    engine_state::{EngineStateAccess, EngineStorageValue},
    Diff,
};
use std::cell::RefCell;

#[derive(Clone, Copy)]
pub struct BatchIO<'db, 'local> {
    pub fallback: EngineStateAccess<'db, 'db, 'db>,
    pub cumulative_diff: &'local Diff,
    pub current_diff: &'local RefCell<Diff>,
}

impl<'db> IO for BatchIO<'db, '_> {
    type StorageValue = EngineStorageValue<'db>;

    fn read_input(&self) -> Self::StorageValue {
        self.fallback.read_input()
    }

    fn return_output(&mut self, value: &[u8]) {
        self.fallback.return_output(value)
    }

    fn read_storage(&self, key: &[u8]) -> Option<Self::StorageValue> {
        if let Some(diff) = self
            .current_diff
            .borrow()
            .get(key)
            .or_else(|| self.cumulative_diff.get(key))
        {
            return diff
                .value()
                .map(|bytes| EngineStorageValue::Vec(bytes.to_vec()));
        }
        self.fallback.read_storage(key)
    }

    fn storage_has_key(&self, key: &[u8]) -> bool {
        self.read_storage(key).is_some()
    }

    fn write_storage(&mut self, key: &[u8], value: &[u8]) -> Option<Self::StorageValue> {
        let original_value = self.read_storage(key);

        self.current_diff
            .borrow_mut()
            .modify(key.to_vec(), value.to_vec());

        original_value
    }

    fn write_storage_direct(
        &mut self,
        key: &[u8],
        value: Self::StorageValue,
    ) -> Option<Self::StorageValue> {
        self.write_storage(key, value.as_ref())
    }

    fn remove_storage(&mut self, key: &[u8]) -> Option<Self::StorageValue> {
        let original_value = self.read_storage(key);

        self.current_diff.borrow_mut().delete(key.to_vec());

        original_value
    }
}
