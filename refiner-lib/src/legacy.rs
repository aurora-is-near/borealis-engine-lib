use aurora_engine::parameters::{ResultLog, SubmitResult, TransactionStatus};
use aurora_engine_types::types::RawU256;
use aurora_refiner_types::aurora_block::HashchainOutputKind;
use borsh::{BorshDeserialize, BorshSerialize};
use std::io::Result;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct SubmitResultLegacyV1 {
    pub status: TransactionStatus,
    pub gas_used: u64,
    pub logs: Vec<ResultLog>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ResultLogV1 {
    pub topics: Vec<RawU256>,
    pub data: Vec<u8>,
}

impl From<ResultLogV1> for ResultLog {
    fn from(result: ResultLogV1) -> Self {
        Self {
            address: Default::default(),
            topics: result.topics,
            data: result.data,
        }
    }
}

impl From<SubmitResultLegacyV1> for SubmitResult {
    fn from(result: SubmitResultLegacyV1) -> Self {
        Self::new(result.status, result.gas_used, result.logs)
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct SubmitResultLegacyV2 {
    pub status: TransactionStatus,
    pub gas_used: u64,
    pub logs: Vec<ResultLogV1>,
}

impl From<SubmitResultLegacyV2> for SubmitResult {
    fn from(result: SubmitResultLegacyV2) -> Self {
        Self::new(
            result.status,
            result.gas_used,
            result.logs.into_iter().map(Into::into).collect(),
        )
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct SubmitResultLegacyV3 {
    pub status: bool,
    pub gas_used: u64,
    pub result: Vec<u8>,
    pub logs: Vec<ResultLogV1>,
}

impl From<SubmitResultLegacyV3> for SubmitResult {
    fn from(result: SubmitResultLegacyV3) -> Self {
        let status = if result.status {
            TransactionStatus::Succeed(result.result)
        } else if !result.result.is_empty() {
            TransactionStatus::Revert(result.result)
        } else {
            TransactionStatus::OutOfFund
        };
        Self::new(
            status,
            result.gas_used,
            result.logs.into_iter().map(Into::into).collect(),
        )
    }
}

pub fn to_v1_logs(logs: &[ResultLog]) -> Vec<ResultLogV1> {
    logs.iter()
        .map(|l| ResultLogV1 {
            topics: l.topics.clone(),
            data: l.data.clone(),
        })
        .collect()
}

pub fn decode_submit_result(result: &[u8]) -> Result<(SubmitResult, HashchainOutputKind)> {
    SubmitResult::try_from_slice(result)
        .map(|x| {
            let tag = (&x.status).into();
            (x, HashchainOutputKind::SubmitResultV7(tag))
        })
        .or_else(|_| {
            SubmitResultLegacyV1::try_from_slice(result).map(|x| {
                let tag = (&x.status).into();
                (x.into(), HashchainOutputKind::SubmitResultLegacyV1(tag))
            })
        })
        .or_else(|_| {
            SubmitResultLegacyV2::try_from_slice(result).map(|x| {
                let tag = (&x.status).into();
                (x.into(), HashchainOutputKind::SubmitResultLegacyV2(tag))
            })
        })
        .or_else(|_| {
            SubmitResultLegacyV3::try_from_slice(result)
                .map(|x| (x.into(), HashchainOutputKind::SubmitResultLegacyV3))
        })
}

#[cfg(test)]
mod tests {
    use super::decode_submit_result;

    #[test]
    fn test_legacy_may_2021() {
        // `SubmitResult` taken from
        // https://explorer.mainnet.near.org/transactions/CeG24XrGneQb3PF5xmgzkPGPcFZ3yDzKJ755ZPdXAT6Q#B36aGoLRkspLkjGPgR13ZqUtR3vK7WftqT6HH2BJu5r2
        let data = hex::decode(
            "01b026010000000000140000008a778c47d1d6b4dd5d2cef9881f889c250cd882000000000",
        )
        .unwrap();
        decode_submit_result(&data).unwrap();
    }

    #[test]
    fn test_legacy_v2() {
        let data = vec![
            0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 56, 116, 63, 29, 248, 67, 111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 93, 154, 79, 251, 77, 90, 248, 222, 218, 123, 108,
            150, 1, 0, 0, 0, 0, 0, 5, 0, 0, 0, 2, 0, 0, 0, 225, 255, 252, 196, 146, 61, 4, 181, 89,
            244, 210, 154, 139, 252, 108, 218, 4, 235, 91, 13, 60, 70, 7, 81, 194, 64, 44, 92, 92,
            201, 16, 156, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 44, 180, 94, 219, 69, 23, 213, 148,
            122, 253, 227, 190, 171, 249, 90, 88, 37, 6, 133, 139, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 56, 116, 63, 29, 248, 67, 111,
            3, 0, 0, 0, 221, 242, 82, 173, 27, 226, 200, 155, 105, 194, 176, 104, 252, 55, 141,
            170, 149, 43, 167, 241, 99, 196, 161, 22, 40, 245, 90, 77, 245, 35, 179, 239, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 44, 180, 94, 219, 69, 23, 213, 148, 122, 253, 227, 190, 171,
            249, 90, 88, 37, 6, 133, 139, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 99, 218, 77, 182,
            239, 78, 124, 98, 22, 138, 176, 57, 130, 57, 159, 149, 136, 252, 209, 152, 32, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 56, 116, 63,
            29, 248, 67, 111, 3, 0, 0, 0, 221, 242, 82, 173, 27, 226, 200, 155, 105, 194, 176, 104,
            252, 55, 141, 170, 149, 43, 167, 241, 99, 196, 161, 22, 40, 245, 90, 77, 245, 35, 179,
            239, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 99, 218, 77, 182, 239, 78, 124, 98, 22, 138,
            176, 57, 130, 57, 159, 149, 136, 252, 209, 152, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            188, 20, 67, 196, 112, 198, 19, 14, 209, 5, 39, 72, 225, 121, 253, 49, 62, 95, 32, 244,
            32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 93, 154,
            79, 251, 77, 90, 248, 222, 218, 123, 1, 0, 0, 0, 28, 65, 30, 154, 150, 224, 113, 36,
            28, 47, 33, 247, 114, 107, 23, 174, 137, 227, 202, 180, 199, 139, 229, 14, 6, 43, 3,
            169, 255, 251, 186, 209, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 18, 145, 110, 142, 189, 165, 239, 163, 83, 72, 238, 98, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 239, 156, 232, 99, 35, 227, 34,
            176, 3, 0, 0, 0, 215, 138, 217, 95, 164, 108, 153, 75, 101, 81, 208, 218, 133, 252, 39,
            95, 230, 19, 206, 55, 101, 127, 184, 213, 227, 209, 48, 132, 1, 89, 216, 34, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 44, 180, 94, 219, 69, 23, 213, 148, 122, 253, 227, 190, 171,
            249, 90, 88, 37, 6, 133, 139, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 188, 20, 67, 196,
            112, 198, 19, 14, 209, 5, 39, 72, 225, 121, 253, 49, 62, 95, 32, 244, 128, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 56,
            116, 63, 29, 248, 67, 111, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 4, 93, 154, 79, 251, 77, 90, 248, 222, 218, 123, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        decode_submit_result(&data).unwrap();
    }
}
