use std::{ffi::OsString, process::Command};

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;

/// Run a Cheers app through Dioxus/Subsecond hot-patching.
///
/// This command is intentionally a thin supervisor over `dx serve --hot-patch`.
/// It enables the dependency feature `cheers/subsecond` for the served app.
#[derive(Debug, Parser)]
pub(crate) struct SubsecondArgs {
    /// Arguments passed to `dx serve`. Put them after `--`.
    ///
    /// Example: `cargo cheers subsecond -- --bin app --port 8080`.
    #[arg(last = true, value_name = "DX_SERVE_ARGS")]
    args: Vec<OsString>,
}

pub(crate) fn run(args: SubsecondArgs) -> Result<()> {
    let mut command = Command::new("dx");
    command.args(build_dx_serve_args(&args)?);

    let status = command
        .status()
        .context("failed to run `dx serve`; install dioxus-cli or run through `nix shell nixpkgs#dioxus-cli`")?;
    if !status.success() {
        return Err(anyhow!("Subsecond dev server exited with {status}"));
    }

    Ok(())
}

fn build_dx_serve_args(args: &SubsecondArgs) -> Result<Vec<OsString>> {
    reject_addr_args(&args.args)?;

    let mut out = vec![
        OsString::from("serve"),
        OsString::from("--addr"),
        OsString::from("127.0.0.1"),
        OsString::from("--hot-patch"),
        OsString::from("--hot-reload"),
        OsString::from("true"),
        OsString::from("--server"),
        OsString::from("--open"),
        OsString::from("false"),
        OsString::from("--features"),
        OsString::from("cheers/subsecond"),
    ];

    #[cfg(target_os = "linux")]
    out.push(OsString::from(
        "--rustc-args=-Clink-arg=-Wl,--export-dynamic",
    ));

    out.extend(args.args.iter().cloned());
    Ok(out)
}

fn reject_addr_args(args: &[OsString]) -> Result<()> {
    for arg in args {
        if arg == "--addr" || arg.to_str().is_some_and(|arg| arg.starts_with("--addr=")) {
            bail!("`cargo cheers subsecond` always serves on 127.0.0.1; remove `--addr`");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> SubsecondArgs {
        SubsecondArgs {
            args: values.iter().map(OsString::from).collect(),
        }
    }

    #[test]
    fn defaults_to_server_hot_patch_and_subsecond_feature() {
        let built = build_dx_serve_args(&args(&["--manifest-path", "Cargo.toml"]))
            .expect("args should build");
        let built = built
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(built.starts_with(&[
            "serve".to_owned(),
            "--addr".to_owned(),
            "127.0.0.1".to_owned(),
            "--hot-patch".to_owned(),
            "--hot-reload".to_owned(),
            "true".to_owned(),
            "--server".to_owned(),
            "--open".to_owned(),
            "false".to_owned(),
            "--features".to_owned(),
            "cheers/subsecond".to_owned(),
        ]));
        assert!(built.contains(&"--manifest-path".to_owned()));
    }

    #[test]
    fn appends_user_args_after_required_dx_args() {
        let built = build_dx_serve_args(&args(&["--package", "cheers-example-realtime"]))
            .expect("args should build");
        let built = built
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert!(built.ends_with(&["--package".to_owned(), "cheers-example-realtime".to_owned(),]));
    }

    #[test]
    fn addr_arg_is_rejected() {
        let err = build_dx_serve_args(&args(&["--addr", "127.0.0.1"]))
            .expect_err("addr override should be rejected")
            .to_string();

        assert!(err.contains("always serves on 127.0.0.1"));
    }

    #[test]
    fn addr_equals_arg_is_rejected() {
        let err = build_dx_serve_args(&args(&["--addr=127.0.0.1"]))
            .expect_err("addr override should be rejected")
            .to_string();

        assert!(err.contains("always serves on 127.0.0.1"));
    }
}
