use std::fs;
use std::io::{self, BufRead};
use std::path::Path;

const FILEPATH_PLACEHOLDER: &'static str = "<secrets_file>";
const KEY_VAL_DELIM: char = '=';

#[derive(Debug)]
pub struct Secret {
    pub db_url: String,
    pub api_key: String,
}

pub fn read_secrets<T>(secrets_path: T) -> crate::Result<Secret>
where
    T: AsRef<Path>,
{
    let file_name = secrets_path
        .as_ref()
        .to_str()
        .unwrap_or(FILEPATH_PLACEHOLDER);

    let file =
        fs::File::open(&secrets_path).map_err(|_| format!("'{file_name}' path not found"))?;
    let file_buf = io::BufReader::new(file);

    parse_secrets(file_buf, file_name)
}

/// The actual parsing part lives here for testability
fn parse_secrets<R>(file: R, file_name: &str) -> crate::Result<Secret>
where
    R: BufRead,
{
    let mut db_url: Option<String> = None;
    let mut api_key: Option<String> = None;

    for (i, line) in file.lines().enumerate() {
        let i = i + 1;
        let line = line?;

        match line.split_once(KEY_VAL_DELIM) {
            Some((key, value)) => match key.trim() {
                "DB_URL" => db_url = Some(check_val_empty(value, file_name, i)?),
                "API_KEY" => api_key = Some(check_val_empty(value, file_name, i)?),
                _ => return Err(format!("unexpected key '{key}' at {file_name}:{i}").into()),
            },
            None => return Err(format!("invalid line format at {file_name}:{i}").into()),
        }
    }

    let db_url = db_url.ok_or_else(|| format!("DB_URL value not found in {file_name}"))?;
    let api_key = api_key.ok_or_else(|| format!("API_KEY value not found in {file_name}"))?;

    Ok(Secret { db_url, api_key })
}

fn check_val_empty(value: &str, file: &str, line: usize) -> crate::Result<String> {
    if value.is_empty() {
        return Err(format!("value is empty at {file}:{line}").into());
    }
    Ok(value.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn mock_file(contents: &str) -> Cursor<Vec<u8>> {
        Cursor::new(contents.as_bytes().to_vec())
    }

    #[test]
    fn test_read_secrets_success() {
        let file_content = "DB_URL=http://localhost:1234
API_KEY=myapikey";
        let file = mock_file(file_content);

        // Read secrets (passing the mock file)
        let result = parse_secrets(file, FILEPATH_PLACEHOLDER);

        assert!(result.is_ok());
        let Secret { db_url, api_key } = result.unwrap();
        assert_eq!(db_url, "http://localhost:1234");
        assert_eq!(api_key, "myapikey");
    }

    #[test]
    fn test_read_secrets_missing_db_url() {
        let file_content = r"API_KEY=myapikey";
        let file = mock_file(file_content);

        // Read secrets and assert error for missing DB_URL
        let result = parse_secrets(file, FILEPATH_PLACEHOLDER);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "DB_URL value not found in <secrets_file>"
        );
    }

    #[test]
    fn test_read_secrets_missing_api_key() {
        let file_content = r"DB_URL=http://localhost:1234";
        let file = mock_file(file_content);

        // Read secrets and assert error for missing API_KEY
        let result = parse_secrets(file, FILEPATH_PLACEHOLDER);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "API_KEY value not found in <secrets_file>"
        );
    }

    #[test]
    fn test_read_secrets_invalid_key() {
        let file_content = r"DB_URL=http://localhost:1234
INVALID_KEY=value
API_KEY=myapikey";
        let file = mock_file(file_content);

        // Read secrets and assert error for invalid line format
        let result = parse_secrets(file, FILEPATH_PLACEHOLDER);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "unexpected key 'INVALID_KEY' at <secrets_file>:2"
        );
    }

    #[test]
    fn test_read_secrets_invalid_line_format() {
        let file_content = r"DB_URL=http://localhost:1234
API_KEY:myapikey";
        let file = mock_file(file_content);

        // Read secrets and assert error for invalid line format
        let result = parse_secrets(file, FILEPATH_PLACEHOLDER);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "invalid line format at <secrets_file>:2"
        );
    }

    #[test]
    fn test_read_secrets_empty_value() {
        let file_content = r"DB_URL=
API_KEY=myapikey";
        let file = mock_file(file_content);

        // Read secrets and assert error for empty DB_URL value
        let result = parse_secrets(file, FILEPATH_PLACEHOLDER);

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "value is empty at <secrets_file>:1"
        );
    }

    // It's quite difficult to consistently test the case where the file is not found...
}
