use std::process::Output;
use std::str;

/// Merges the stdout and stderr of a process into a `String`.
pub fn whole_output(output: &Output) -> Result<String, str::Utf8Error> {
    let stdout_str = str::from_utf8(&output.stdout)?;
    let stderr_str = str::from_utf8(&output.stderr)?;
    Ok(format!("{}\n{}", stdout_str, stderr_str))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn whole_output_works() {
        let out = Command::new("ls")
            .arg("README.md")
            .arg("nonexistent.exe")
            .output()
            .expect("ls ran");
        let result = whole_output(&out).expect("output is utf8");
        assert!(result.contains("README.md"));
        assert!(result.contains("nonexistent.exe"));
        assert!(result.contains("No such file"));
    }
}
