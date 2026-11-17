/**
 * Parses config.
 *
 * # Errors
 * Returns an error when the file cannot be read.
 */
pub fn parse_config(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|err| err.to_string())
}