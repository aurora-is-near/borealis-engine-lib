use vergen_git2::{Emitter, Git2Builder};

fn main() -> anyhow::Result<()> {
    let git2 = Git2Builder::all_git()?;
    Emitter::default().add_instructions(&git2)?.emit()?;
    Ok(())
}
