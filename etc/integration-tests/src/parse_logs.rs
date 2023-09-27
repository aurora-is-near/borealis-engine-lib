//! Helper functions for parsing information from nearcore logs.

use crate::ansi_utils;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};

/// Extracts the block height from a nearcore stats log line. Example:
/// 2023-08-01T08:26:02.869336Z  INFO stats: #      52 24LGJniHqZjL4nhd55ocngvTofG8jyTKQNCgixcTsJTV Validator | 4 validators 3 peers ⬇ 4.92 kB/s ⬆ 4.80 kB/s 1.90 bps 0 gas/s CPU: 0%, Mem: 59.9 MB
/// Notably there are 8 characters available for the block height after the '#' in the line.
fn get_height_from_log(line: &str) -> anyhow::Result<u32> {
    let index = line
        .find('#')
        .ok_or_else(|| anyhow::Error::msg("Unknown nearcore log format"))?;
    let height_str = &line[(index + 1)..(index + 9)];
    let height = height_str.trim().parse()?;
    Ok(height)
}

/// Waits for the stats log to report a block height greater than the given value.
pub async fn wait_for_height<R: AsyncRead + Unpin + Send>(
    stdout: R,
    expected_height: u32,
) -> anyhow::Result<()> {
    let mut reader = BufReader::new(stdout).lines();
    while let Some(line) = reader.next_line().await? {
        let ascii_only = ansi_utils::strip_ansi(line);
        if !ascii_only.contains("stats:") {
            continue;
        }
        let height = get_height_from_log(&ascii_only)?;
        if height > expected_height {
            break;
        }
    }
    Ok(())
}
