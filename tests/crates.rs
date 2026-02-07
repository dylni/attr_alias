use std::io;
use std::process::Command;

fn test_crate(name: &str, message: &str) -> io::Result<()> {
    let dir = file!().strip_suffix(".rs").expect("missing extension");
    let output = Command::new("cargo")
        .args(["run", message])
        .current_dir([dir, "/", name].concat())
        .output()?;

    assert_eq!(Some(0), output.status.code());
    assert_eq!(message.as_bytes(), output.stdout);

    Ok(())
}

#[test]
fn test_complex() -> io::Result<()> {
    test_crate("complex", "test")
}

#[test]
fn test_dependency() -> io::Result<()> {
    fn test(message: &str) -> io::Result<()> {
        test_crate("dependent", message)
    }

    test("1")?;
    test("2")
}
