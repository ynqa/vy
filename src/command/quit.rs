#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Quit;

pub(crate) fn parse(arguments: &str) -> anyhow::Result<Quit> {
    if arguments.is_empty() {
        Ok(Quit)
    } else {
        Err(anyhow::anyhow!("quit does not accept arguments"))
    }
}
