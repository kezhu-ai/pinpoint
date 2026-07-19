//! Shell init snippets + clipboard helper.

use std::io::Write;
use std::process::{Command, Stdio};

use anyhow::Result;

use crate::Shell;

pub fn copy_to_clipboard(s: &str) -> Result<()> {
    let candidates: &[(&str, &[&str])] = if cfg!(target_os = "windows") {
        &[("clip", &[])]
    } else if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else {
        &[("wl-copy", &[]), ("xclip", &["-selection", "clipboard"])]
    };
    for (bin, args) in candidates {
        if let Ok(mut child) = Command::new(bin)
            .args(*args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(s.as_bytes()).ok();
                drop(stdin);
                let _ = child.wait();
                return Ok(());
            }
        }
    }
    println!("(no clipboard helper found; printed to stdout instead)\n");
    print!("{}", s);
    if !s.ends_with('\n') { println!(); }
    Ok(())
}

pub fn emit_init(shell: Shell) {
    match shell {
        Shell::Bash | Shell::Zsh => {
            // One-line save: `pp "my prompt"` -> saved
            // Multi-line: `pp "line 1" "line 2" ...` -> joined with space
            println!("# Add to ~/.bashrc or ~/.zshrc:");
            println!("#   echo \"fix the auth bug in auth.ts\" | pp");
            println!("#   pp \"remember: format timestamps as ISO\" --tags ts");
            println!("#   pp-fix() {{ pp \"$1\" --tags fix; }}");
            println!();
            println!("pp() {{");
            println!("    if [ $# -eq 0 ]; then");
            println!("        # stdin mode");
            println!("        command pinpoint log --source \"${{PP_SOURCE:-shell}}\"");
            println!("    else");
            println!("        # arg mode: join all args, drop leading `--` if present");
            println!("        local args=(\"$@\")");
            println!("        command pinpoint log \"${{args[*]}}\" --source \"${{PP_SOURCE:-shell}}\"");
            println!("    fi");
            println!("}}");
        }
        Shell::Fish => {
            println!("# Add to ~/.config/fish/config.fish:");
            println!("function pp");
            println!("    if test (count $argv) -eq 0");
            println!("        command pinpoint log --source \"$PP_SOURCE\"");
            println!("    else");
            println!("        command pinpoint log \"$argv\" --source \"$PP_SOURCE\"");
            println!("    end");
            println!("end");
        }
    }
}