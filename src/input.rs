use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::Result;

pub fn read_input(path: Option<&Path>) -> Result<(String, Option<PathBuf>)> {
    if let Some(path) = path {
        let content = fs::read_to_string(path)?;
        return Ok((content, Some(path.to_path_buf())));
    }

    let mut stdin = std::io::stdin();
    let mut content = String::new();
    stdin.read_to_string(&mut content)?;
    Ok((content, None))
}
