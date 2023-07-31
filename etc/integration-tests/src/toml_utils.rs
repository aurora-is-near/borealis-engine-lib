/// Get a value from a toml Table of Tables by recursively looking up values from the
/// Tables using the provided list of keys (`path`).
pub fn toml_recursive_get<'a>(
    table: &'a toml::Table,
    path: &[&str],
) -> anyhow::Result<&'a toml::Value> {
    let first_key = path
        .first()
        .ok_or_else(|| anyhow::Error::msg("Empty toml lookup path"))?;
    let mut current_value = table
        .get(*first_key)
        .ok_or_else(|| anyhow::anyhow!("Key {first_key} not found in toml table"))?;
    for key in &path[1..] {
        let current_table = current_value.as_table().ok_or_else(|| {
            anyhow::anyhow!("Cannot look up {key} because toml value is not a Table")
        })?;
        current_value = current_table
            .get(*key)
            .ok_or_else(|| anyhow::anyhow!("Key {key} not found in toml table"))?;
    }
    Ok(current_value)
}
