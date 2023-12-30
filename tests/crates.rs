use std::io;
use std::process::Command;

fn test(message: &str) -> io::Result<()> {
    let dir = file!().strip_suffix(".rs").expect("missing extension");
    let output = Command::new("cargo")
        .args(["run", message])
        .current_dir([dir, "/dependent"].concat())
        .output()?;

    assert_eq!(Some(0), output.status.code());
    assert_eq!(message.as_bytes(), output.stdout);

    Ok(())
}

#[test]
fn test_simple() -> io::Result<()> {
    test("1")?;
    test("2")
}
