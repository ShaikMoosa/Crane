use crane_application::{Command, CommandResult, Library, LibrarySnapshot, TransferIo};
use crane_platform::SystemEnvironment;
use crane_storage::JsonRepository;
use std::path::Path;

pub fn open_library(path: &Path) -> Result<Library, String> {
    Library::open(
        Box::new(JsonRepository::new(path.to_path_buf())),
        Box::new(SystemEnvironment::new()),
    )
    .map_err(|error| error.to_string())
}

pub fn open_library_with_transfers(
    path: &Path,
    transfers: Box<dyn TransferIo>,
) -> Result<Library, String> {
    open_library(path).map(|library| library.with_transfers(transfers))
}

pub fn execute(path: &Path, command: Command) -> Result<(CommandResult, LibrarySnapshot), String> {
    let mut library = open_library(path)?;
    let result = library
        .execute(command)
        .map_err(|error| error.to_string())?;
    let snapshot = library.snapshot();
    Ok((result, snapshot))
}
