use std::path::Path;

use anyhow::Result;
use assert_cmd::cargo;
use assert_fs::{NamedTempFile, prelude::*};
use predicates::prelude::*;
use pretty_assertions::assert_eq;

pub(crate) fn write_named_temp_file(name: &str, content: &str) -> Result<NamedTempFile> {
    let file = NamedTempFile::new(name)?;
    file.write_str(content)?;
    Ok(file)
}

pub(crate) fn assert_file_contents(path: impl AsRef<Path>, expected: &str) -> Result<()> {
    assert_eq!(std::fs::read_to_string(path.as_ref())?, expected);
    Ok(())
}

pub(crate) fn assert_formatted_file(
    name: &str,
    input: &str,
    expected: &str,
    extra_args: &[&str],
) -> Result<()> {
    let file = write_named_temp_file(name, input)?;

    let mut cmd = cargo::cargo_bin_cmd!("cargo-cheers");
    cmd.arg("fmt").args(extra_args).arg(file.path());

    cmd.assert().success();
    assert_file_contents(file.path(), expected)
}

pub(crate) fn assert_formatted_stdin(
    input: &str,
    expected: &str,
    extra_args: &[&str],
) -> Result<()> {
    let file = write_named_temp_file("stdin", input)?;

    let mut cmd = cargo::cargo_bin_cmd!("cargo-cheers");
    cmd.arg("fmt").args(extra_args).arg("-s").pipe_stdin(file)?;

    cmd.assert()
        .success()
        .stdout(predicate::str::diff(expected.to_owned()));

    Ok(())
}
